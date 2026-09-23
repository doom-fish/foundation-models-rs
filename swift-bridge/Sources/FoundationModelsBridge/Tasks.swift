import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

final class FMTaskHandle: @unchecked Sendable {
    private let lock = NSLock()
    private var task: Task<Void, Never>?
    private var cancelRequested = false

    fileprivate func attach(_ task: Task<Void, Never>) {
        lock.lock()
        self.task = task
        let cancel = cancelRequested
        lock.unlock()
        if cancel {
            task.cancel()
        }
    }

    func cancel() {
        lock.lock()
        cancelRequested = true
        let task = self.task
        lock.unlock()
        task?.cancel()
    }

    fileprivate var isCancelRequested: Bool {
        lock.lock()
        defer { lock.unlock() }
        return cancelRequested
    }

    fileprivate func wait() async {
        lock.lock()
        let task = self.task
        lock.unlock()
        await task?.value
    }
}

final class FMRequestGate: @unchecked Sendable {
    private let lock = NSLock()
    private var running: [ObjectIdentifier: FMTaskHandle] = [:]

    fileprivate func begin(_ handle: FMTaskHandle) -> [FMTaskHandle] {
        lock.lock()
        defer { lock.unlock() }
        let cancelled = running.values.filter { $0.isCancelRequested }
        running[ObjectIdentifier(handle)] = handle
        return cancelled
    }

    fileprivate func end(_ handle: FMTaskHandle) {
        lock.lock()
        running[ObjectIdentifier(handle)] = nil
        lock.unlock()
    }
}

func startBridgeTask(
    gate: FMRequestGate? = nil,
    _ body: @escaping @Sendable () async -> Void
) -> UnsafeMutableRawPointer {
    let handle = FMTaskHandle()
    let cancelledEarlier = gate?.begin(handle) ?? []
    let task = Task.detached {
        for earlier in cancelledEarlier {
            await earlier.wait()
        }
        await body()
        gate?.end(handle)
    }
    handle.attach(task)
    return Unmanaged.passRetained(handle).toOpaque()
}

@_cdecl("fm_task_cancel")
public func fm_task_cancel(_ taskPtr: UnsafeMutableRawPointer?) {
    guard let taskPtr else { return }
    Unmanaged<FMTaskHandle>.fromOpaque(taskPtr).takeUnretainedValue().cancel()
}

@_cdecl("fm_object_retain")
public func fm_object_retain(_ ptr: UnsafeMutableRawPointer?) {
    guard let ptr else { return }
    _ = Unmanaged<AnyObject>.fromOpaque(ptr).retain()
}

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
@available(macOS 26.0, *)
final class SessionBox: @unchecked Sendable {
    let session: LanguageModelSession
    let gate = FMRequestGate()

    init(_ session: LanguageModelSession) {
        self.session = session
    }
}

@available(macOS 26.0, *)
func sessionBox(from ptr: UnsafeMutableRawPointer) -> SessionBox {
    Unmanaged<SessionBox>.fromOpaque(ptr).takeUnretainedValue()
}
#endif
