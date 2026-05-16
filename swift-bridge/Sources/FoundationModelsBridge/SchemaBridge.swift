import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
@available(macOS 26.0, *)
func bridgeBuildDynamicSchema(from json: Any, name: String) throws -> DynamicGenerationSchema {
    guard let dict = json as? [String: Any] else {
        throw NSError(domain: "fm-bridge", code: -1, userInfo: [
            NSLocalizedDescriptionKey: "schema must be a JSON object"
        ])
    }

    if let reference = dict["$ref"] as? String ?? dict["reference"] as? String {
        return DynamicGenerationSchema(referenceTo: reference)
    }

    let typeStr = (dict["type"] as? String) ?? "object"
    let description = dict["description"] as? String
    let schemaName = (dict["name"] as? String) ?? name

    switch typeStr {
    case "object":
        let propertyDict = (dict["properties"] as? [String: Any]) ?? [:]
        let properties = try propertyDict
            .sorted { $0.key < $1.key }
            .map { propertyName, propertyValue -> DynamicGenerationSchema.Property in
                let propertySchema = try bridgeBuildDynamicSchema(from: propertyValue, name: propertyName)
                let propertyDescription = (propertyValue as? [String: Any])?["description"] as? String
                let isOptional = ((propertyValue as? [String: Any])?["optional"] as? Bool) ?? false
                return DynamicGenerationSchema.Property(
                    name: propertyName,
                    description: propertyDescription,
                    schema: propertySchema,
                    isOptional: isOptional
                )
            }
        return DynamicGenerationSchema(name: schemaName, description: description, properties: properties)
    case "array":
        var itemJSON: Any = dict["items"] ?? ["type": "string"]
        var minimumElements = dict["min"] as? Int
        var maximumElements = dict["max"] as? Int
        if let guides = dict["guides"] as? [Any] {
            for guideValue in guides {
                let guide = try guideDictionary(guideValue)
                switch guide["kind"] as? String {
                case "minimum_count":
                    if let count = guide["value"] as? Int {
                        minimumElements = count
                    }
                case "maximum_count":
                    if let count = guide["value"] as? Int {
                        maximumElements = count
                    }
                case "count":
                    if let count = guide["value"] as? Int {
                        minimumElements = count
                        maximumElements = count
                    } else {
                        if let minimum = guide["min"] as? Int {
                            minimumElements = minimum
                        }
                        if let maximum = guide["max"] as? Int {
                            maximumElements = maximum
                        }
                    }
                case "element":
                    guard let nestedGuide = guide["guide"] else {
                        throw NSError(domain: "fm-bridge", code: -9, userInfo: [
                            NSLocalizedDescriptionKey: "element guides must include a nested guide"
                        ])
                    }
                    guard var itemDict = itemJSON as? [String: Any] else {
                        throw NSError(domain: "fm-bridge", code: -10, userInfo: [
                            NSLocalizedDescriptionKey: "array items must be schema objects"
                        ])
                    }
                    var itemGuides = (itemDict["guides"] as? [Any]) ?? []
                    itemGuides.append(nestedGuide)
                    itemDict["guides"] = itemGuides
                    itemJSON = itemDict
                default:
                    throw NSError(domain: "fm-bridge", code: -11, userInfo: [
                        NSLocalizedDescriptionKey: "unsupported array guide"
                    ])
                }
            }
        }
        let itemSchema = try bridgeBuildDynamicSchema(from: itemJSON, name: "Item")
        return DynamicGenerationSchema(
            arrayOf: itemSchema,
            minimumElements: minimumElements,
            maximumElements: maximumElements
        )
    case "any_of":
        let choices = (dict["choices"] as? [Any]) ?? []
        if choices.allSatisfy({ $0 is String }) {
            return DynamicGenerationSchema(
                name: schemaName,
                description: description,
                anyOf: choices.compactMap { $0 as? String }
            )
        }
        return DynamicGenerationSchema(
            name: schemaName,
            description: description,
            anyOf: try choices.enumerated().map { index, element in
                try bridgeBuildDynamicSchema(from: element, name: "Choice\(index)")
            }
        )
    case "string":
        return DynamicGenerationSchema(
            type: String.self,
            guides: try stringGuides(from: dict["guides"] as? [Any])
        )
    case "integer":
        return DynamicGenerationSchema(
            type: Int.self,
            guides: try intGuides(from: dict["guides"] as? [Any])
        )
    case "float":
        return DynamicGenerationSchema(
            type: Float.self,
            guides: try floatGuides(from: dict["guides"] as? [Any])
        )
    case "number", "double":
        return DynamicGenerationSchema(
            type: Double.self,
            guides: try doubleGuides(from: dict["guides"] as? [Any])
        )
    case "decimal":
        return DynamicGenerationSchema(
            type: Decimal.self,
            guides: try decimalGuides(from: dict["guides"] as? [Any])
        )
    case "boolean":
        return DynamicGenerationSchema(type: Bool.self, guides: [])
    case "generated_content":
        return DynamicGenerationSchema(type: GeneratedContent.self, guides: [])
    default:
        throw NSError(domain: "fm-bridge", code: -2, userInfo: [
            NSLocalizedDescriptionKey: "unsupported schema type: \(typeStr)"
        ])
    }
}

@available(macOS 26.0, *)
private func guideDictionary(_ value: Any) throws -> [String: Any] {
    guard let dict = value as? [String: Any], let kind = dict["kind"] as? String else {
        throw NSError(domain: "fm-bridge", code: -3, userInfo: [
            NSLocalizedDescriptionKey: "guide entries must be objects with a kind"
        ])
    }
    var result = dict
    result["kind"] = kind
    return result
}

@available(macOS 26.0, *)
private func stringGuides(from values: [Any]?) throws -> [GenerationGuide<String>] {
    guard let values else { return [] }
    return try values.map { value in
        let dict = try guideDictionary(value)
        switch dict["kind"] as? String {
        case "constant":
            return .constant((dict["value"] as? String) ?? "")
        case "any_of":
            return .anyOf((dict["values"] as? [String]) ?? [])
        case "pattern":
            let pattern = (dict["pattern"] as? String) ?? ".*"
            return .pattern(try Regex(pattern))
        default:
            throw NSError(domain: "fm-bridge", code: -4, userInfo: [
                NSLocalizedDescriptionKey: "unsupported string guide"
            ])
        }
    }
}

@available(macOS 26.0, *)
private func intGuides(from values: [Any]?) throws -> [GenerationGuide<Int>] {
    guard let values else { return [] }
    return try values.map { value in
        let dict = try guideDictionary(value)
        switch dict["kind"] as? String {
        case "minimum":
            return .minimum(dict["value"] as? Int ?? 0)
        case "maximum":
            return .maximum(dict["value"] as? Int ?? 0)
        case "range":
            return .range((dict["min"] as? Int ?? 0)...(dict["max"] as? Int ?? 0))
        default:
            throw NSError(domain: "fm-bridge", code: -5, userInfo: [
                NSLocalizedDescriptionKey: "unsupported integer guide"
            ])
        }
    }
}

@available(macOS 26.0, *)
private func floatGuides(from values: [Any]?) throws -> [GenerationGuide<Float>] {
    guard let values else { return [] }
    return try values.map { value in
        let dict = try guideDictionary(value)
        switch dict["kind"] as? String {
        case "minimum":
            return .minimum(Float(dict["value"] as? Double ?? 0))
        case "maximum":
            return .maximum(Float(dict["value"] as? Double ?? 0))
        case "range":
            return .range(
                Float(dict["min"] as? Double ?? 0)...Float(dict["max"] as? Double ?? 0)
            )
        default:
            throw NSError(domain: "fm-bridge", code: -6, userInfo: [
                NSLocalizedDescriptionKey: "unsupported float guide"
            ])
        }
    }
}

@available(macOS 26.0, *)
private func doubleGuides(from values: [Any]?) throws -> [GenerationGuide<Double>] {
    guard let values else { return [] }
    return try values.map { value in
        let dict = try guideDictionary(value)
        switch dict["kind"] as? String {
        case "minimum":
            return .minimum(dict["value"] as? Double ?? 0)
        case "maximum":
            return .maximum(dict["value"] as? Double ?? 0)
        case "range":
            return .range((dict["min"] as? Double ?? 0)...(dict["max"] as? Double ?? 0))
        default:
            throw NSError(domain: "fm-bridge", code: -7, userInfo: [
                NSLocalizedDescriptionKey: "unsupported number guide"
            ])
        }
    }
}

@available(macOS 26.0, *)
private func decimalGuides(from values: [Any]?) throws -> [GenerationGuide<Decimal>] {
    guard let values else { return [] }
    return try values.map { value in
        let dict = try guideDictionary(value)
        func decimal(_ raw: Any?) -> Decimal {
            if let number = raw as? NSNumber {
                return number.decimalValue
            }
            if let string = raw as? String, let decimal = Decimal(string: string) {
                return decimal
            }
            return .zero
        }
        switch dict["kind"] as? String {
        case "minimum":
            return .minimum(decimal(dict["value"]))
        case "maximum":
            return .maximum(decimal(dict["value"]))
        case "range":
            return .range(decimal(dict["min"])...decimal(dict["max"]))
        default:
            throw NSError(domain: "fm-bridge", code: -8, userInfo: [
                NSLocalizedDescriptionKey: "unsupported decimal guide"
            ])
        }
    }
}
#endif

@_cdecl("fm_generation_schema_compile_json")
public func fm_generation_schema_compile_json(
    _ requestJSON: UnsafePointer<CChar>,
    _ context: UnsafeMutableRawPointer?,
    _ callback: @convention(c) (
        UnsafeMutableRawPointer?,
        UnsafeMutablePointer<CChar>?,
        UnsafeMutablePointer<CChar>?,
        Int32
    ) -> Void
) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        do {
            guard let data = String(cString: requestJSON).data(using: .utf8),
                  let parsed = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let rootJSON = parsed["root"] else {
                callback(context, nil, ffiString("schema request JSON is invalid"), FM_INVALID_ARGUMENT)
                return
            }
            let root = try bridgeBuildDynamicSchema(from: rootJSON, name: "Root")
            let dependencyJSON = (parsed["dependencies"] as? [Any]) ?? []
            let dependencies = try dependencyJSON.enumerated().map { index, value in
                try bridgeBuildDynamicSchema(from: value, name: "Dependency\(index)")
            }
            let schema = try GenerationSchema(root: root, dependencies: dependencies)
            let encoded = try encodeBridge(schema)
            callback(context, ffiString(encoded), nil, FM_OK)
            return
        } catch {
            let (code, message) = mapError(error)
            callback(context, nil, ffiString(message), code)
            return
        }
    }
    #endif
    callback(context, nil, ffiString("FoundationModels requires macOS 26.0 or newer"), FM_MODEL_UNAVAILABLE)
}

@_cdecl("fm_generation_schema_validate_json")
public func fm_generation_schema_validate_json(
    _ schemaJSON: UnsafePointer<CChar>,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        do {
            _ = try decodeGenerationSchema(from: String(cString: schemaJSON))
            return FM_OK
        } catch {
            let (code, message) = mapError(error)
            writeErrorOut(errorOut, message)
            return code
        }
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return FM_MODEL_UNAVAILABLE
}
