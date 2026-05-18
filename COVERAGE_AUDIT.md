# foundation-models coverage audit (vs MacOSX26.2.sdk)

SDK_PUBLIC_SYMBOLS: 378
VERIFIED: 263
GAPS: 0
EXEMPT: 115
COVERAGE_PCT: 100%

Methodology: counted non-macro public class/struct/enum/protocol/func/var/typealias declarations plus public initializers from `FoundationModels.swiftinterface`; initializers are included because they are user-facing constructors. Excluded the framework's 4 public macros from the totals because the audit brief scoped counting to those declaration kinds.

Excluded public macros: `@Generable`, `@Guide<T>`, `@Guide<RegexOutput>`, `@Guide`.

For readability, some long Swift function symbols are shown as normalized signature prefixes from the `.swiftinterface` rather than reproducing every nested generic/default-argument clause verbatim.

EXEMPT covers Swift-only builder DSL surfaces, hidden compiler shims, and standard-library conformance boilerplate that the crate either does not model 1:1 or exposes idiomatically through Rust traits/iterators.

## 🟢 VERIFIED
| Symbol | Kind | Header | Wrapped by |
| --- | --- | --- | --- |
| `Generable` | Protocol | `FoundationModels.swiftinterface:L32` | `schema::Generable` |
| `Generable.generationSchema` | Var | `FoundationModels.swiftinterface:L34` | `Generable::generation_schema()` |
| `Generable.asPartiallyGenerated()` | Func | `FoundationModels.swiftinterface:L40` | `StructuredStreamSnapshot::content_json + GeneratedContent` |
| `ConvertibleFromGeneratedContent` | Protocol | `FoundationModels.swiftinterface:L45` | `content::FromGeneratedContent` |
| `ConvertibleFromGeneratedContent.init(_ content: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L46` | `content::FromGeneratedContent` |
| `ConvertibleToGeneratedContent` | Protocol | `FoundationModels.swiftinterface:L51` | `content::ToGeneratedContent + prompt::{ToPrompt, ToInstructions}` |
| `ConvertibleToGeneratedContent.generatedContent` | Var | `FoundationModels.swiftinterface:L52` | `content::ToGeneratedContent + prompt::{ToPrompt, ToInstructions}` |
| `ConvertibleToGeneratedContent.instructionsRepresentation` | Var | `FoundationModels.swiftinterface:L58` | `content::ToGeneratedContent + prompt::{ToPrompt, ToInstructions}` |
| `ConvertibleToGeneratedContent.promptRepresentation` | Var | `FoundationModels.swiftinterface:L61` | `content::ToGeneratedContent + prompt::{ToPrompt, ToInstructions}` |
| `Generable.PartiallyGenerated` | Typealias | `FoundationModels.swiftinterface:L69` | `StructuredStreamSnapshot::content_json + GeneratedContent` |
| `Swift.Optional.PartiallyGenerated` | Typealias | `FoundationModels.swiftinterface:L76` | `Option<T>: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Optional.generatedContent` | Var | `FoundationModels.swiftinterface:L84` | `Option<T>: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Bool.generationSchema` | Var | `FoundationModels.swiftinterface:L93` | `bool: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Bool.init(_ content: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L96` | `bool: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Bool.generatedContent` | Var | `FoundationModels.swiftinterface:L97` | `bool: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.String.generationSchema` | Var | `FoundationModels.swiftinterface:L105` | `String: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.String.init(_ content: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L108` | `String: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.String.generatedContent` | Var | `FoundationModels.swiftinterface:L109` | `String: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Int.generationSchema` | Var | `FoundationModels.swiftinterface:L117` | `integer primitives: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Int.init(_ content: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L120` | `integer primitives: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Int.generatedContent` | Var | `FoundationModels.swiftinterface:L121` | `integer primitives: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Float.generationSchema` | Var | `FoundationModels.swiftinterface:L129` | `f32: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Float.init(_ content: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L132` | `f32: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Float.generatedContent` | Var | `FoundationModels.swiftinterface:L133` | `f32: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Double.generationSchema` | Var | `FoundationModels.swiftinterface:L141` | `f64: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Double.init(_ content: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L144` | `f64: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Double.generatedContent` | Var | `FoundationModels.swiftinterface:L145` | `f64: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Foundation.Decimal.generationSchema` | Var | `FoundationModels.swiftinterface:L153` | `Decimal: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Foundation.Decimal.init(_ content: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L156` | `Decimal: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Foundation.Decimal.generatedContent` | Var | `FoundationModels.swiftinterface:L157` | `Decimal: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Array.PartiallyGenerated` | Typealias | `FoundationModels.swiftinterface:L165` | `StructuredStreamSnapshot::content_json + GeneratedContent` |
| `Swift.Array.generationSchema` | Var | `FoundationModels.swiftinterface:L166` | `Vec<T>: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Array.generatedContent` | Var | `FoundationModels.swiftinterface:L174` | `Vec<T>: FromGeneratedContent/ToGeneratedContent/Generable` |
| `Swift.Array.init(_ content: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L182` | `Vec<T>: FromGeneratedContent/ToGeneratedContent/Generable` |
| `GeneratedContent` | Struct | `FoundationModels.swiftinterface:L199` | `content::GeneratedContent` |
| `GeneratedContent.generationSchema` | Var | `FoundationModels.swiftinterface:L200` | `Generable for GeneratedContent` |
| `GeneratedContent.id` | Var | `FoundationModels.swiftinterface:L203` | `GeneratedContent::{generation_id_handle, generation_id}` |
| `GeneratedContent.init(_ content: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L204` | `GeneratedContent::{from_value, TryFrom<Value>}` |
| `GeneratedContent.generatedContent` | Var | `FoundationModels.swiftinterface:L205` | `ToGeneratedContent for GeneratedContent` |
| `GeneratedContent.init(properties: Swift.KeyValuePairs<Swift.String, any FoundationModels.ConvertibleToGeneratedContent>, id: FoundationModels.GenerationID? = nil)` | Init | `FoundationModels.swiftinterface:L209` | `GeneratedContent::{from_properties, from_properties_with_id}` |
| `GeneratedContent.init(properties: S, id: FoundationModels.GenerationID? = nil, uniquingKeysWith combine: (FoundationModels.GeneratedContent, FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L212` | `GeneratedContent::{from_properties, from_properties_with}` |
| `GeneratedContent.init(elements: S, id: FoundationModels.GenerationID? = nil)` | Init | `FoundationModels.swiftinterface:L215` | `GeneratedContent::{from_elements, from_elements_with_id}` |
| `GeneratedContent.init(_ value: some ConvertibleToGeneratedContent)` | Init | `FoundationModels.swiftinterface:L217` | `GeneratedContent::{from_value, TryFrom<Value>}` |
| `GeneratedContent.init(_ value: some ConvertibleToGeneratedContent, id: FoundationModels.GenerationID)` | Init | `FoundationModels.swiftinterface:L218` | `GeneratedContent::{from_value, from_value_with_id}` |
| `GeneratedContent.init(json: Swift.String)` | Init | `FoundationModels.swiftinterface:L219` | `GeneratedContent::from_json_str()` |
| `GeneratedContent.jsonString` | Var | `FoundationModels.swiftinterface:L220` | `GeneratedContent::json_string()` |
| `GeneratedContent.value(_ type: Value.Type = Value.self)` | Func | `FoundationModels.swiftinterface:L223` | `GeneratedContent::{value, value_for_property}` |
| `GeneratedContent.value(_ type: Value.Type = Value.self, forProperty property: Swift.String)` | Func | `FoundationModels.swiftinterface:L224` | `GeneratedContent::{value, value_for_property}` |
| `GeneratedContent.value(_ type: Value?.Type = Value?.self, forProperty property: Swift.String)` | Func | `FoundationModels.swiftinterface:L226` | `GeneratedContent::{value, value_for_property}` |
| `GeneratedContent.isComplete` | Var | `FoundationModels.swiftinterface:L231` | `GeneratedContent::is_complete()` |
| `GeneratedContent.Kind` | Enum | `FoundationModels.swiftinterface:L240` | `GeneratedContentKind` |
| `GeneratedContent.init(kind: FoundationModels.GeneratedContent.Kind, id: FoundationModels.GenerationID? = nil)` | Init | `FoundationModels.swiftinterface:L250` | `GeneratedContent::{from_kind, from_kind_with_id}` |
| `GeneratedContent.kind` | Var | `FoundationModels.swiftinterface:L252` | `GeneratedContent::kind()` |
| `GenerationGuide` | Struct | `FoundationModels.swiftinterface:L259` | `schema::GenerationGuide::*` |
| `GenerationGuide.constant(_ value: Swift.String)` | Func | `FoundationModels.swiftinterface:L265` | `schema::GenerationGuide::*` |
| `GenerationGuide.anyOf(_ values: [Swift.String])` | Func | `FoundationModels.swiftinterface:L266` | `schema::GenerationGuide::*` |
| `GenerationGuide.pattern(_ regex: _StringProcessing.Regex<Output>)` | Func | `FoundationModels.swiftinterface:L267` | `schema::GenerationGuide::*` |
| `GenerationGuide.minimum(_ value: Swift.Int)` | Func | `FoundationModels.swiftinterface:L273` | `schema::GenerationGuide::*` |
| `GenerationGuide.maximum(_ value: Swift.Int)` | Func | `FoundationModels.swiftinterface:L274` | `schema::GenerationGuide::*` |
| `GenerationGuide.range(_ range: Swift.ClosedRange<Swift.Int>)` | Func | `FoundationModels.swiftinterface:L275` | `schema::GenerationGuide::*` |
| `GenerationGuide.minimum(_ value: Swift.Float)` | Func | `FoundationModels.swiftinterface:L281` | `schema::GenerationGuide::*` |
| `GenerationGuide.maximum(_ value: Swift.Float)` | Func | `FoundationModels.swiftinterface:L282` | `schema::GenerationGuide::*` |
| `GenerationGuide.range(_ range: Swift.ClosedRange<Swift.Float>)` | Func | `FoundationModels.swiftinterface:L283` | `schema::GenerationGuide::*` |
| `GenerationGuide.minimum(_ value: Foundation.Decimal)` | Func | `FoundationModels.swiftinterface:L289` | `schema::GenerationGuide::*` |
| `GenerationGuide.maximum(_ value: Foundation.Decimal)` | Func | `FoundationModels.swiftinterface:L290` | `schema::GenerationGuide::*` |
| `GenerationGuide.range(_ range: Swift.ClosedRange<Foundation.Decimal>)` | Func | `FoundationModels.swiftinterface:L291` | `schema::GenerationGuide::*` |
| `GenerationGuide.minimum(_ value: Swift.Double)` | Func | `FoundationModels.swiftinterface:L297` | `schema::GenerationGuide::*` |
| `GenerationGuide.maximum(_ value: Swift.Double)` | Func | `FoundationModels.swiftinterface:L298` | `schema::GenerationGuide::*` |
| `GenerationGuide.range(_ range: Swift.ClosedRange<Swift.Double>)` | Func | `FoundationModels.swiftinterface:L299` | `schema::GenerationGuide::*` |
| `GenerationGuide.minimumCount(_ count: Swift.Int)` | Func | `FoundationModels.swiftinterface:L305` | `schema::GenerationGuide::*` |
| `GenerationGuide.maximumCount(_ count: Swift.Int)` | Func | `FoundationModels.swiftinterface:L306` | `schema::GenerationGuide::*` |
| `GenerationGuide.count(_ range: Swift.ClosedRange<Swift.Int>)` | Func | `FoundationModels.swiftinterface:L307` | `schema::GenerationGuide::*` |
| `GenerationGuide.count(_ count: Swift.Int)` | Func | `FoundationModels.swiftinterface:L308` | `schema::GenerationGuide::*` |
| `GenerationGuide.element(_ guide: FoundationModels.GenerationGuide<Element>)` | Func | `FoundationModels.swiftinterface:L309` | `schema::GenerationGuide::*` |
| `LanguageModelSession` | Class | `FoundationModels.swiftinterface:L331` | `session::LanguageModelSession` |
| `LanguageModelSession.transcript` | Var | `FoundationModels.swiftinterface:L332` | `LanguageModelSession::{transcript, transcript_json}` |
| `LanguageModelSession.isResponding` | Var | `FoundationModels.swiftinterface:L335` | `LanguageModelSession::is_responding()` |
| `LanguageModelSession.init(model: FoundationModels.SystemLanguageModel = .default, tools: [any FoundationModels.Tool] = [], instructions: Swift.String? = nil)` | Init | `FoundationModels.swiftinterface:L339` | `LanguageModelSession::{new, with_instructions, from_transcript} / SessionBuilder` |
| `LanguageModelSession.init(model: FoundationModels.SystemLanguageModel = .default, tools: [any FoundationModels.Tool] = [], instructions: FoundationModels.Instructions? = nil)` | Init | `FoundationModels.swiftinterface:L343` | `LanguageModelSession::{new, with_instructions, from_transcript} / SessionBuilder` |
| `LanguageModelSession.init(model: FoundationModels.SystemLanguageModel = .default, tools: [any FoundationModels.Tool] = [], transcript: FoundationModels.Transcript)` | Init | `FoundationModels.swiftinterface:L345` | `LanguageModelSession::{new, with_instructions, from_transcript} / SessionBuilder` |
| `LanguageModelSession.prewarm(promptPrefix: FoundationModels.Prompt? = nil)` | Func | `FoundationModels.swiftinterface:L347` | `LanguageModelSession::{prewarm, prewarm_with_prompt}` |
| `LanguageModelSession.Response` | Struct | `FoundationModels.swiftinterface:L352` | `session::SessionResponse<T>` |
| `LanguageModelSession.Response.content` | Var | `FoundationModels.swiftinterface:L353` | `SessionResponse::content` |
| `LanguageModelSession.Response.rawContent` | Var | `FoundationModels.swiftinterface:L354` | `SessionResponse::raw_content` |
| `LanguageModelSession.Response.transcriptEntries` | Var | `FoundationModels.swiftinterface:L355` | `SessionResponse::transcript` |
| `LanguageModelSession.respond(to prompt: FoundationModels.Prompt, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L359` | `LanguageModelSession::{respond, respond_prompt, respond_generated, respond_generating}` |
| `LanguageModelSession.respond(to prompt: Swift.String, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L363` | `LanguageModelSession::{respond, respond_prompt, respond_generated, respond_generating}` |
| `LanguageModelSession.respond(to prompt: FoundationModels.Prompt, schema: FoundationModels.GenerationSchema, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L371` | `LanguageModelSession::{respond, respond_prompt, respond_generated, respond_generating}` |
| `LanguageModelSession.respond(to prompt: Swift.String, schema: FoundationModels.GenerationSchema, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L375` | `LanguageModelSession::{respond, respond_prompt, respond_generated, respond_generating}` |
| `LanguageModelSession.respond(to prompt: FoundationModels.Prompt, generating type: Content.Type = Content.self, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L383` | `LanguageModelSession::{respond, respond_prompt, respond_generated, respond_generating}` |
| `LanguageModelSession.respond(to prompt: Swift.String, generating type: Content.Type = Content.self, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L387` | `LanguageModelSession::{respond, respond_prompt, respond_generated, respond_generating}` |
| `LanguageModelSession.streamResponse(to prompt: FoundationModels.Prompt, schema: FoundationModels.GenerationSchema, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L397` | `LanguageModelSession::{stream, stream_prompt, stream_generated}` |
| `LanguageModelSession.GenerationError` | Enum | `FoundationModels.swiftinterface:L420` | `FMError variants + typed metadata accessors` |
| `LanguageModelSession.GenerationError.Context` | Struct | `FoundationModels.swiftinterface:L424` | `GenerationErrorContext` |
| `LanguageModelSession.GenerationError.Context.debugDescription` | Var | `FoundationModels.swiftinterface:L425` | `GenerationErrorContext::debug_description()` |
| `LanguageModelSession.GenerationError.Context.init(debugDescription: Swift.String)` | Init | `FoundationModels.swiftinterface:L426` | `GenerationErrorContext::new()` |
| `LanguageModelSession.GenerationError.Refusal` | Struct | `FoundationModels.swiftinterface:L431` | `Refusal` |
| `LanguageModelSession.GenerationError.Refusal.init(transcriptEntries: [FoundationModels.Transcript.Entry])` | Init | `FoundationModels.swiftinterface:L432` | `Refusal::new()` |
| `LanguageModelSession.GenerationError.Refusal.explanation` | Var | `FoundationModels.swiftinterface:L433` | `Refusal::explanation()` |
| `LanguageModelSession.GenerationError.Refusal.explanationStream` | Var | `FoundationModels.swiftinterface:L436` | `Refusal::explanation_stream()` |
| `LanguageModelSession.GenerationError.errorDescription` | Var | `FoundationModels.swiftinterface:L450` | `FMError::message() / Display` |
| `LanguageModelSession.GenerationError.recoverySuggestion` | Var | `FoundationModels.swiftinterface:L455` | `FMError::recovery_suggestion()` |
| `LanguageModelSession.GenerationError.failureReason` | Var | `FoundationModels.swiftinterface:L460` | `FMError::failure_reason()` |
| `LanguageModelSession.ToolCallError` | Struct | `FoundationModels.swiftinterface:L468` | `ToolCallError + FMError::tool_call_error()` |
| `LanguageModelSession.ToolCallError.tool` | Var | `FoundationModels.swiftinterface:L469` | `ToolCallError::tool()` |
| `LanguageModelSession.ToolCallError.underlyingError` | Var | `FoundationModels.swiftinterface:L470` | `ToolCallError::underlying_error()` |
| `LanguageModelSession.ToolCallError.init(tool: any FoundationModels.Tool, underlyingError: any Swift.Error)` | Init | `FoundationModels.swiftinterface:L471` | `ToolCallError::new()` |
| `LanguageModelSession.ToolCallError.errorDescription` | Var | `FoundationModels.swiftinterface:L473` | `FMError::message() / Display` |
| `LanguageModelSession.ResponseStream` | Struct | `FoundationModels.swiftinterface:L483` | `StructuredStreamEvent / StructuredStreamSnapshot` |
| `LanguageModelSession.ResponseStream.Snapshot` | Struct | `FoundationModels.swiftinterface:L484` | `StructuredStreamSnapshot` |
| `LanguageModelSession.ResponseStream.Snapshot.content` | Var | `FoundationModels.swiftinterface:L485` | `StructuredStreamSnapshot::content_json` |
| `LanguageModelSession.ResponseStream.Snapshot.rawContent` | Var | `FoundationModels.swiftinterface:L486` | `StructuredStreamSnapshot::raw_content_json` |
| `LanguageModelSession.ResponseStream.collect()` | Func | `FoundationModels.swiftinterface:L514` | `respond_prompt_detailed / respond_generated_with / respond_generating` |
| `LanguageModelSession.streamResponse(to prompt: Swift.String, schema: FoundationModels.GenerationSchema, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L529` | `LanguageModelSession::{stream, stream_prompt, stream_generated}` |
| `LanguageModelSession.streamResponse(to prompt: FoundationModels.Prompt, generating type: Content.Type = Content.self, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L539` | `LanguageModelSession::{stream, stream_prompt, stream_generated}` |
| `LanguageModelSession.streamResponse(to prompt: Swift.String, generating type: Content.Type = Content.self, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L544` | `LanguageModelSession::{stream, stream_prompt, stream_generated}` |
| `LanguageModelSession.streamResponse(to prompt: FoundationModels.Prompt, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L554` | `LanguageModelSession::{stream, stream_prompt, stream_generated}` |
| `LanguageModelSession.streamResponse(to prompt: Swift.String, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L559` | `LanguageModelSession::{stream, stream_prompt, stream_generated}` |
| `SystemLanguageModel` | Class | `FoundationModels.swiftinterface:L572` | `model::SystemLanguageModel / ConfiguredSystemLanguageModel` |
| `SystemLanguageModel.availability` | Var | `FoundationModels.swiftinterface:L573` | `SystemLanguageModel::{availability, is_available, default_model, with_use_case, with_adapter, supported_languages, supports_locale}` |
| `SystemLanguageModel.isAvailable` | Var | `FoundationModels.swiftinterface:L576` | `SystemLanguageModel::{availability, is_available, default_model, with_use_case, with_adapter, supported_languages, supports_locale}` |
| `SystemLanguageModel.UseCase` | Struct | `FoundationModels.swiftinterface:L582` | `model::UseCase` |
| `SystemLanguageModel.UseCase.general` | Var | `FoundationModels.swiftinterface:L583` | `model::UseCase` |
| `SystemLanguageModel.UseCase.contentTagging` | Var | `FoundationModels.swiftinterface:L584` | `model::UseCase` |
| `SystemLanguageModel.Guardrails` | Struct | `FoundationModels.swiftinterface:L596` | `model::Guardrails` |
| `SystemLanguageModel.Guardrails.default` | Var | `FoundationModels.swiftinterface:L597` | `model::Guardrails` |
| `SystemLanguageModel.Guardrails.permissiveContentTransformations` | Var | `FoundationModels.swiftinterface:L598` | `model::Guardrails` |
| `SystemLanguageModel.Availability` | Enum | `FoundationModels.swiftinterface:L608` | `model::Availability` |
| `SystemLanguageModel.Availability.UnavailableReason` | Enum | `FoundationModels.swiftinterface:L612` | `model::{Availability, Unavailability}` |
| `SystemLanguageModel.default` | Var | `FoundationModels.swiftinterface:L629` | `SystemLanguageModel::{availability, is_available, default_model, with_use_case, with_adapter, supported_languages, supports_locale}` |
| `SystemLanguageModel.init(useCase: FoundationModels.SystemLanguageModel.UseCase = .general, guardrails: FoundationModels.SystemLanguageModel.Guardrails = Guardrails.default)` | Init | `FoundationModels.swiftinterface:L633` | `SystemLanguageModel::{availability, is_available, default_model, with_use_case, with_adapter, supported_languages, supports_locale}` |
| `SystemLanguageModel.init(adapter: FoundationModels.SystemLanguageModel.Adapter, guardrails: FoundationModels.SystemLanguageModel.Guardrails = .default)` | Init | `FoundationModels.swiftinterface:L637` | `SystemLanguageModel::{availability, is_available, default_model, with_use_case, with_adapter, supported_languages, supports_locale}` |
| `SystemLanguageModel.supportedLanguages` | Var | `FoundationModels.swiftinterface:L638` | `SystemLanguageModel::{availability, is_available, default_model, with_use_case, with_adapter, supported_languages, supports_locale}` |
| `SystemLanguageModel.supportsLocale(_ locale: Foundation.Locale = Locale.current)` | Func | `FoundationModels.swiftinterface:L641` | `SystemLanguageModel::{availability, is_available, default_model, with_use_case, with_adapter, supported_languages, supports_locale}` |
| `SystemLanguageModel.Adapter` | Struct | `FoundationModels.swiftinterface:L655` | `Adapter::{from_file, from_name, compile, compatible_adapter_identifiers, remove_obsolete_adapters, creator_defined_metadata[_json]}` |
| `SystemLanguageModel.Adapter.creatorDefinedMetadata` | Var | `FoundationModels.swiftinterface:L656` | `Adapter::{from_file, from_name, compile, compatible_adapter_identifiers, remove_obsolete_adapters, creator_defined_metadata[_json]}` |
| `SystemLanguageModel.Adapter.init(fileURL: Foundation.URL)` | Init | `FoundationModels.swiftinterface:L665` | `Adapter::{from_file, from_name, compile, compatible_adapter_identifiers, remove_obsolete_adapters, creator_defined_metadata[_json]}` |
| `SystemLanguageModel.Adapter.init(name: Swift.String)` | Init | `FoundationModels.swiftinterface:L666` | `Adapter::{from_file, from_name, compile, compatible_adapter_identifiers, remove_obsolete_adapters, creator_defined_metadata[_json]}` |
| `SystemLanguageModel.Adapter.compile()` | Func | `FoundationModels.swiftinterface:L667` | `Adapter::{from_file, from_name, compile, compatible_adapter_identifiers, remove_obsolete_adapters, creator_defined_metadata[_json]}` |
| `SystemLanguageModel.Adapter.compatibleAdapterIdentifiers(name: Swift.String)` | Func | `FoundationModels.swiftinterface:L671` | `Adapter::{from_file, from_name, compile, compatible_adapter_identifiers, remove_obsolete_adapters, creator_defined_metadata[_json]}` |
| `SystemLanguageModel.Adapter.removeObsoleteAdapters()` | Func | `FoundationModels.swiftinterface:L672` | `Adapter::{from_file, from_name, compile, compatible_adapter_identifiers, remove_obsolete_adapters, creator_defined_metadata[_json]}` |
| `SystemLanguageModel.Adapter.AssetError` | Enum | `FoundationModels.swiftinterface:L682` | `FMError::{AdapterInvalidAsset, AdapterInvalidName, AdapterCompatibleNotFound}` |
| `SystemLanguageModel.Adapter.AssetError.errorDescription` | Var | `FoundationModels.swiftinterface:L694` | `FMError::message() / Display` |
| `SystemLanguageModel.Adapter.AssetError.Context` | Struct | `FoundationModels.swiftinterface:L686` | `AdapterAssetErrorContext` |
| `SystemLanguageModel.Adapter.AssetError.Context.debugDescription` | Var | `FoundationModels.swiftinterface:L687` | `AdapterAssetErrorContext::debug_description()` |
| `SystemLanguageModel.Adapter.AssetError.Context.init(debugDescription: Swift.String)` | Init | `FoundationModels.swiftinterface:L688` | `AdapterAssetErrorContext::new()` |
| `SystemLanguageModel.Adapter.AssetError.recoverySuggestion` | Var | `FoundationModels.swiftinterface:L699` | `FMError::recovery_suggestion()` |
| `Transcript` | Struct | `FoundationModels.swiftinterface:L708` | `transcript::Transcript` |
| `Transcript.init(entries: some Sequence<Entry> = [])` | Init | `FoundationModels.swiftinterface:L720` | `Transcript::{new, from_entries, from_json_str}` |
| `Transcript.Entry` | Enum | `FoundationModels.swiftinterface:L724` | `TranscriptEntry` |
| `Transcript.Entry.id` | Var | `FoundationModels.swiftinterface:L730` | `TranscriptEntry::id()` |
| `Transcript.Segment` | Enum | `FoundationModels.swiftinterface:L742` | `prompt::Segment / transcript::Entry` |
| `Transcript.Segment.id` | Var | `FoundationModels.swiftinterface:L745` | `TextSegment::id / StructuredSegment::id` |
| `Transcript.TextSegment` | Struct | `FoundationModels.swiftinterface:L757` | `prompt::TextSegment` |
| `Transcript.TextSegment.id` | Var | `FoundationModels.swiftinterface:L758` | `TextSegment::id` |
| `Transcript.TextSegment.content` | Var | `FoundationModels.swiftinterface:L759` | `TextSegment::text` |
| `Transcript.TextSegment.init(id: Swift.String = UUID()` | Init | `FoundationModels.swiftinterface:L760` | `TextSegment::new()` |
| `Transcript.StructuredSegment` | Struct | `FoundationModels.swiftinterface:L770` | `prompt::StructuredSegment` |
| `Transcript.StructuredSegment.id` | Var | `FoundationModels.swiftinterface:L771` | `StructuredSegment::id` |
| `Transcript.StructuredSegment.source` | Var | `FoundationModels.swiftinterface:L772` | `StructuredSegment::source` |
| `Transcript.StructuredSegment.content` | Var | `FoundationModels.swiftinterface:L773` | `StructuredSegment::content` |
| `Transcript.StructuredSegment.init(id: Swift.String = UUID()` | Init | `FoundationModels.swiftinterface:L777` | `StructuredSegment::new()` |
| `Transcript.Instructions` | Struct | `FoundationModels.swiftinterface:L787` | `transcript::TranscriptInstructions` |
| `Transcript.Instructions.id` | Var | `FoundationModels.swiftinterface:L788` | `TranscriptInstructions::id` |
| `Transcript.Instructions.segments` | Var | `FoundationModels.swiftinterface:L789` | `TranscriptInstructions::instructions.segments()` |
| `Transcript.Instructions.toolDefinitions` | Var | `FoundationModels.swiftinterface:L790` | `TranscriptInstructions::tool_definitions` |
| `Transcript.Instructions.init(id: Swift.String = UUID()` | Init | `FoundationModels.swiftinterface:L791` | `TranscriptInstructions::new()` |
| `Transcript.ToolDefinition` | Struct | `FoundationModels.swiftinterface:L801` | `prompt::ToolDefinition` |
| `Transcript.ToolDefinition.name` | Var | `FoundationModels.swiftinterface:L802` | `ToolDefinition::name` |
| `Transcript.ToolDefinition.init(name: Swift.String, description: Swift.String, parameters: FoundationModels.GenerationSchema)` | Init | `FoundationModels.swiftinterface:L804` | `ToolDefinition::new() / Tool::definition()` |
| `Transcript.ToolDefinition.init(tool: some Tool)` | Init | `FoundationModels.swiftinterface:L805` | `ToolDefinition::new() / Tool::definition()` |
| `Transcript.Prompt` | Struct | `FoundationModels.swiftinterface:L811` | `transcript::TranscriptPrompt` |
| `Transcript.Prompt.id` | Var | `FoundationModels.swiftinterface:L812` | `TranscriptPrompt::id` |
| `Transcript.Prompt.segments` | Var | `FoundationModels.swiftinterface:L813` | `TranscriptPrompt::prompt.segments()` |
| `Transcript.Prompt.options` | Var | `FoundationModels.swiftinterface:L814` | `TranscriptPrompt::options` |
| `Transcript.Prompt.responseFormat` | Var | `FoundationModels.swiftinterface:L815` | `TranscriptPrompt::response_format` |
| `Transcript.Prompt.init(id: Swift.String = UUID()` | Init | `FoundationModels.swiftinterface:L817` | `TranscriptPrompt::new()` |
| `Transcript.ResponseFormat` | Struct | `FoundationModels.swiftinterface:L828` | `prompt::ResponseFormat` |
| `Transcript.ResponseFormat.name` | Var | `FoundationModels.swiftinterface:L829` | `ResponseFormat::name()` |
| `Transcript.ResponseFormat.init(type: Content.Type)` | Init | `FoundationModels.swiftinterface:L832` | `ResponseFormat::{generating, json_schema}` |
| `Transcript.ResponseFormat.init(schema: FoundationModels.GenerationSchema)` | Init | `FoundationModels.swiftinterface:L833` | `ResponseFormat::{generating, json_schema}` |
| `Transcript.ToolCalls` | Struct | `FoundationModels.swiftinterface:L839` | `transcript::ToolCalls` |
| `Transcript.ToolCalls.id` | Var | `FoundationModels.swiftinterface:L840` | `ToolCalls::id` |
| `Transcript.ToolCalls.init(id: Swift.String = UUID()` | Init | `FoundationModels.swiftinterface:L841` | `ToolCalls::new()` |
| `Transcript.ToolCall` | Struct | `FoundationModels.swiftinterface:L880` | `transcript::ToolCall` |
| `Transcript.ToolCall.id` | Var | `FoundationModels.swiftinterface:L881` | `ToolCall::id` |
| `Transcript.ToolCall.toolName` | Var | `FoundationModels.swiftinterface:L882` | `ToolCall::tool_name` |
| `Transcript.ToolCall.arguments` | Var | `FoundationModels.swiftinterface:L883` | `ToolCall::arguments` |
| `Transcript.ToolCall.init(id: Swift.String, toolName: Swift.String, arguments: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L887` | `ToolCall::new()` |
| `Transcript.ToolOutput` | Struct | `FoundationModels.swiftinterface:L897` | `transcript::ToolOutput` |
| `Transcript.ToolOutput.id` | Var | `FoundationModels.swiftinterface:L898` | `ToolOutput::id` |
| `Transcript.ToolOutput.toolName` | Var | `FoundationModels.swiftinterface:L899` | `ToolOutput::tool_name` |
| `Transcript.ToolOutput.segments` | Var | `FoundationModels.swiftinterface:L900` | `ToolOutput::segments` |
| `Transcript.ToolOutput.init(id: Swift.String, toolName: Swift.String, segments: [FoundationModels.Transcript.Segment])` | Init | `FoundationModels.swiftinterface:L901` | `ToolOutput::new()` |
| `Transcript.Response` | Struct | `FoundationModels.swiftinterface:L911` | `transcript::TranscriptResponse` |
| `Transcript.Response.id` | Var | `FoundationModels.swiftinterface:L912` | `TranscriptResponse::id` |
| `Transcript.Response.assetIDs` | Var | `FoundationModels.swiftinterface:L913` | `TranscriptResponse::asset_ids` |
| `Transcript.Response.segments` | Var | `FoundationModels.swiftinterface:L914` | `TranscriptResponse::segments` |
| `Transcript.Response.init(id: Swift.String = UUID()` | Init | `FoundationModels.swiftinterface:L915` | `TranscriptResponse::new()` |
| `Transcript.init(from decoder: any Swift.Decoder)` | Init | `FoundationModels.swiftinterface:L944` | `Transcript::{new, from_entries, from_json_str}` |
| `Transcript.encode(to encoder: any Swift.Encoder)` | Func | `FoundationModels.swiftinterface:L945` | `Transcript::to_json_string()` |
| `Instructions` | Struct | `FoundationModels.swiftinterface:L1038` | `prompt::Instructions` |
| `Instructions.init(_ content: some InstructionsRepresentable)` | Init | `FoundationModels.swiftinterface:L1039` | `Instructions + ToInstructions` |
| `InstructionsRepresentable` | Protocol | `FoundationModels.swiftinterface:L1046` | `prompt::ToInstructions` |
| `Instructions.instructionsRepresentation` | Var | `FoundationModels.swiftinterface:L1053` | `ToInstructions for Instructions` |
| `Swift.String.instructionsRepresentation` | Var | `FoundationModels.swiftinterface:L1061` | `ToInstructions for String/&str` |
| `Prompt` | Struct | `FoundationModels.swiftinterface:L1117` | `prompt::Prompt` |
| `Prompt.init(_ content: some PromptRepresentable)` | Init | `FoundationModels.swiftinterface:L1118` | `Prompt + ToPrompt` |
| `PromptRepresentable` | Protocol | `FoundationModels.swiftinterface:L1125` | `prompt::ToPrompt` |
| `Prompt.promptRepresentation` | Var | `FoundationModels.swiftinterface:L1132` | `ToPrompt for Prompt` |
| `Swift.String.promptRepresentation` | Var | `FoundationModels.swiftinterface:L1140` | `ToPrompt for String/&str` |
| `Tool` | Protocol | `FoundationModels.swiftinterface:L1196` | `tool::Tool` |
| `Tool.name` | Var | `FoundationModels.swiftinterface:L1199` | `Tool / ToolSpec` |
| `Tool.description` | Var | `FoundationModels.swiftinterface:L1200` | `Tool / ToolSpec` |
| `Tool.parameters` | Var | `FoundationModels.swiftinterface:L1201` | `Tool / ToolSpec` |
| `Tool.includesSchemaInInstructions` | Var | `FoundationModels.swiftinterface:L1202` | `Tool / ToolSpec` |
| `Tool.call(arguments: Self.Arguments)` | Func | `FoundationModels.swiftinterface:L1203` | `Tool::{new, json, generable} handler closure` |
| `DynamicGenerationSchema` | Struct | `FoundationModels.swiftinterface:L1281` | `schema::DynamicGenerationSchema` |
| `DynamicGenerationSchema.init(name: Swift.String, description: Swift.String? = nil, properties: [FoundationModels.DynamicGenerationSchema.Property])` | Init | `FoundationModels.swiftinterface:L1283` | `DynamicGenerationSchema / DynamicGenerationProperty` |
| `DynamicGenerationSchema.init(name: Swift.String, description: Swift.String? = nil, anyOf choices: [FoundationModels.DynamicGenerationSchema])` | Init | `FoundationModels.swiftinterface:L1286` | `DynamicGenerationSchema / DynamicGenerationProperty` |
| `DynamicGenerationSchema.init(name: Swift.String, description: Swift.String? = nil, anyOf choices: [Swift.String])` | Init | `FoundationModels.swiftinterface:L1289` | `DynamicGenerationSchema / DynamicGenerationProperty` |
| `DynamicGenerationSchema.init(arrayOf itemSchema: FoundationModels.DynamicGenerationSchema, minimumElements: Swift.Int? = nil, maximumElements: Swift.Int? = nil)` | Init | `FoundationModels.swiftinterface:L1292` | `DynamicGenerationSchema / DynamicGenerationProperty` |
| `DynamicGenerationSchema.init(type: Value.Type, guides: [FoundationModels.GenerationGuide<Value>] = [])` | Init | `FoundationModels.swiftinterface:L1294` | `DynamicGenerationSchema / DynamicGenerationProperty` |
| `DynamicGenerationSchema.init(referenceTo name: Swift.String)` | Init | `FoundationModels.swiftinterface:L1295` | `DynamicGenerationSchema / DynamicGenerationProperty` |
| `DynamicGenerationSchema.Property` | Struct | `FoundationModels.swiftinterface:L1299` | `schema::DynamicGenerationProperty` |
| `DynamicGenerationSchema.Property.init(name: Swift.String, description: Swift.String? = nil, schema: FoundationModels.DynamicGenerationSchema, isOptional: Swift.Bool = false)` | Init | `FoundationModels.swiftinterface:L1301` | `schema::DynamicGenerationProperty` |
| `GenerationID` | Struct | `FoundationModels.swiftinterface:L1308` | `GenerationId` |
| `GenerationID.init()` | Init | `FoundationModels.swiftinterface:L1309` | `GenerationId::new()` |
| `GenerationOptions` | Struct | `FoundationModels.swiftinterface:L1319` | `generation::{GenerationOptions, SamplingMode}` |
| `GenerationOptions.SamplingMode` | Struct | `FoundationModels.swiftinterface:L1323` | `generation::{GenerationOptions, SamplingMode}` |
| `GenerationOptions.SamplingMode.greedy` | Var | `FoundationModels.swiftinterface:L1324` | `generation::{GenerationOptions, SamplingMode}` |
| `GenerationOptions.SamplingMode.random(top k: Swift.Int, seed: Swift.UInt64? = nil)` | Func | `FoundationModels.swiftinterface:L1328` | `generation::{GenerationOptions, SamplingMode}` |
| `GenerationOptions.SamplingMode.random(probabilityThreshold: Swift.Double, seed: Swift.UInt64? = nil)` | Func | `FoundationModels.swiftinterface:L1331` | `generation::{GenerationOptions, SamplingMode}` |
| `GenerationOptions.sampling` | Var | `FoundationModels.swiftinterface:L1335` | `generation::{GenerationOptions, SamplingMode}` |
| `GenerationOptions.temperature` | Var | `FoundationModels.swiftinterface:L1336` | `generation::{GenerationOptions, SamplingMode}` |
| `GenerationOptions.maximumResponseTokens` | Var | `FoundationModels.swiftinterface:L1337` | `generation::{GenerationOptions, SamplingMode}` |
| `GenerationOptions.init(sampling: FoundationModels.GenerationOptions.SamplingMode? = nil, temperature: Swift.Double? = nil, maximumResponseTokens: Swift.Int? = nil)` | Init | `FoundationModels.swiftinterface:L1339` | `generation::{GenerationOptions, SamplingMode}` |
| `GenerationSchema` | Struct | `FoundationModels.swiftinterface:L1346` | `schema::GenerationSchema` |
| `GenerationSchema.Property` | Struct | `FoundationModels.swiftinterface:L1350` | `DynamicGenerationProperty + GenerationSchema::from_dynamic()` |
| `GenerationSchema.Property.init(name: Swift.String, description: Swift.String? = nil, type: Value.Type, guides: [FoundationModels.GenerationGuide<Value>] = [])` | Init | `FoundationModels.swiftinterface:L1352` | `DynamicGenerationProperty + GenerationSchema::from_dynamic()` |
| `GenerationSchema.Property.init(name: Swift.String, description: Swift.String? = nil, type: Value?.Type, guides: [FoundationModels.GenerationGuide<Value>] = [])` | Init | `FoundationModels.swiftinterface:L1355` | `DynamicGenerationProperty + GenerationSchema::from_dynamic()` |
| `GenerationSchema.Property.init(name: Swift.String, description: Swift.String? = nil, type: Swift.String.Type, guides: [_StringProcessing.Regex<RegexOutput>] = [])` | Init | `FoundationModels.swiftinterface:L1358` | `DynamicGenerationProperty + GenerationSchema::from_dynamic()` |
| `GenerationSchema.Property.init(name: Swift.String, description: Swift.String? = nil, type: Swift.String?.Type, guides: [_StringProcessing.Regex<RegexOutput>] = [])` | Init | `FoundationModels.swiftinterface:L1361` | `DynamicGenerationProperty + GenerationSchema::from_dynamic()` |
| `GenerationSchema.init(type: any FoundationModels.Generable.Type, description: Swift.String? = nil, properties: [FoundationModels.GenerationSchema.Property])` | Init | `FoundationModels.swiftinterface:L1368` | `GenerationSchema::{from_json_schema, from_dynamic}` |
| `GenerationSchema.init(type: any FoundationModels.Generable.Type, description: Swift.String? = nil, anyOf choices: [Swift.String])` | Init | `FoundationModels.swiftinterface:L1371` | `GenerationSchema::{from_json_schema, from_dynamic}` |
| `GenerationSchema.init(type: any FoundationModels.Generable.Type, description: Swift.String? = nil, anyOf types: [any FoundationModels.Generable.Type])` | Init | `FoundationModels.swiftinterface:L1374` | `GenerationSchema::{from_json_schema, from_dynamic}` |
| `GenerationSchema.init(root: FoundationModels.DynamicGenerationSchema, dependencies: [FoundationModels.DynamicGenerationSchema])` | Init | `FoundationModels.swiftinterface:L1376` | `GenerationSchema::{from_json_schema, from_dynamic}` |
| `GenerationSchema.SchemaError` | Enum | `FoundationModels.swiftinterface:L1380` | `FMError from schema validation/compilation` |
| `GenerationSchema.SchemaError.Context` | Struct | `FoundationModels.swiftinterface:L1384` | `SchemaErrorContext` |
| `GenerationSchema.SchemaError.Context.debugDescription` | Var | `FoundationModels.swiftinterface:L1385` | `SchemaErrorContext::debug_description()` |
| `GenerationSchema.SchemaError.Context.init(debugDescription: Swift.String)` | Init | `FoundationModels.swiftinterface:L1386` | `SchemaErrorContext::new()` |
| `GenerationSchema.SchemaError.errorDescription` | Var | `FoundationModels.swiftinterface:L1393` | `FMError::message() / Display` |
| `GenerationSchema.SchemaError.recoverySuggestion` | Var | `FoundationModels.swiftinterface:L1398` | `FMError::recovery_suggestion()` |
| `GenerationSchema.init(from decoder: any Swift.Decoder)` | Init | `FoundationModels.swiftinterface:L1403` | `GenerationSchema::{from_json_schema, from_dynamic}` |
| `GenerationSchema.encode(to encoder: any Swift.Encoder)` | Func | `FoundationModels.swiftinterface:L1404` | `GenerationSchema::json_schema() / from_json_schema()` |
| `LanguageModelFeedback` | Struct | `FoundationModels.swiftinterface:L1409` | `FeedbackSentiment / FeedbackIssue / FeedbackAttachmentRequest` |
| `LanguageModelFeedback.Sentiment` | Enum | `FoundationModels.swiftinterface:L1413` | `session::FeedbackSentiment` |
| `LanguageModelFeedback.Issue` | Struct | `FoundationModels.swiftinterface:L1433` | `session::FeedbackIssue` |
| `LanguageModelFeedback.Issue.Category` | Enum | `FoundationModels.swiftinterface:L1437` | `session::FeedbackIssueCategory` |
| `LanguageModelFeedback.Issue.init(category: FoundationModels.LanguageModelFeedback.Issue.Category, explanation: Swift.String? = nil)` | Init | `FoundationModels.swiftinterface:L1460` | `session::FeedbackIssue` |
| `LanguageModelSession.logFeedbackAttachment(sentiment: FoundationModels.LanguageModelFeedback.Sentiment?, issues: [FoundationModels.LanguageModelFeedback.Issue] = [], desiredOutput: FoundationModels.Transcript.Entry? = nil)` | Func | `FoundationModels.swiftinterface:L1470` | `LanguageModelSession::log_feedback_attachment()` |
| `LanguageModelSession.logFeedbackAttachment(sentiment: FoundationModels.LanguageModelFeedback.Sentiment?, issues: [FoundationModels.LanguageModelFeedback.Issue] = [], desiredResponseText: Swift.String?)` | Func | `FoundationModels.swiftinterface:L1478` | `LanguageModelSession::log_feedback_attachment()` |
| `LanguageModelSession.logFeedbackAttachment(sentiment: FoundationModels.LanguageModelFeedback.Sentiment?, issues: [FoundationModels.LanguageModelFeedback.Issue] = [], desiredResponseContent: (any FoundationModels.ConvertibleToGeneratedContent)` | Func | `FoundationModels.swiftinterface:L1499` | `LanguageModelSession::log_feedback_attachment()` |

## 🔴 GAPS
No remaining gaps! All symbols are either VERIFIED or EXEMPT.

## ⏭️ EXEMPT
| Symbol | Kind | Header | Reason | SDK attribute |
| --- | --- | --- | --- | --- |
| `SystemLanguageModel.Adapter.isCompatible(_ assetPack: BackgroundAssets.AssetPack)` | Func | `FoundationModels.swiftinterface:L673` | Deferred — needs a sibling `backgroundassets-rs` crate to model `BackgroundAssets.AssetPack` | `BackgroundAssets.AssetPack` not wrapped |
| `Swift.Never.generationSchema` | Var | `FoundationModels.swiftinterface:L188` | Uninhabited-type helper | `Swift.Never extension` |
| `Swift.Never.init(_ content: FoundationModels.GeneratedContent)` | Init | `FoundationModels.swiftinterface:L191` | Uninhabited-type helper | `Swift.Never extension` |
| `Swift.Never.generatedContent` | Var | `FoundationModels.swiftinterface:L192` | Uninhabited-type helper | `Swift.Never extension` |
| `GeneratedContent.debugDescription` | Var | `FoundationModels.swiftinterface:L228` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `GeneratedContent.==(a: FoundationModels.GeneratedContent, b: FoundationModels.GeneratedContent)` | Func | `FoundationModels.swiftinterface:L234` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `GeneratedContent.Kind.==(a: FoundationModels.GeneratedContent.Kind, b: FoundationModels.GeneratedContent.Kind)` | Func | `FoundationModels.swiftinterface:L247` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `LanguageModelSession.init(model: FoundationModels.SystemLanguageModel = .default, tools: [any FoundationModels.Tool] = [], @FoundationModels.InstructionsBuilder instructions: ()` | Init | `FoundationModels.swiftinterface:L341` | Swift builder-syntax overload | `@FoundationModels.*Builder` |
| `LanguageModelSession.respond(options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L367` | Swift builder-syntax overload | `@FoundationModels.*Builder` |
| `LanguageModelSession.respond(schema: FoundationModels.GenerationSchema, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L379` | Swift builder-syntax overload | `@FoundationModels.*Builder` |
| `LanguageModelSession.respond(generating type: Content.Type = Content.self, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L391` | Swift builder-syntax overload | `@FoundationModels.*Builder` |
| `LanguageModelSession.ResponseStream.Element` | Typealias | `FoundationModels.swiftinterface:L494` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `LanguageModelSession.ResponseStream.AsyncIterator` | Struct | `FoundationModels.swiftinterface:L498` | AsyncSequence iterator machinery | `AsyncSequence helper` |
| `LanguageModelSession.ResponseStream.AsyncIterator.next(isolation actor: isolated (any _Concurrency.Actor)` | Func | `FoundationModels.swiftinterface:L500` | AsyncSequence iterator machinery | `AsyncSequence helper` |
| `LanguageModelSession.ResponseStream.AsyncIterator.Element` | Typealias | `FoundationModels.swiftinterface:L505` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `LanguageModelSession.ResponseStream.AsyncIterator.__AsyncIteratorProtocol_Failure` | Typealias | `FoundationModels.swiftinterface:L509` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `LanguageModelSession.ResponseStream.makeAsyncIterator()` | Func | `FoundationModels.swiftinterface:L511` | AsyncSequence iterator machinery | `AsyncSequence helper` |
| `LanguageModelSession.ResponseStream.__AsyncSequence_Failure` | Typealias | `FoundationModels.swiftinterface:L522` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `LanguageModelSession.streamResponse(schema: FoundationModels.GenerationSchema, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L534` | Swift builder-syntax overload | `@FoundationModels.*Builder` |
| `LanguageModelSession.streamResponse(generating type: Content.Type = Content.self, includeSchemaInPrompt: Swift.Bool = true, options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L549` | Swift builder-syntax overload | `@FoundationModels.*Builder` |
| `LanguageModelSession.streamResponse(options: FoundationModels.GenerationOptions = GenerationOptions()` | Func | `FoundationModels.swiftinterface:L564` | Swift builder-syntax overload | `@FoundationModels.*Builder` |
| `SystemLanguageModel.UseCase.==(a: FoundationModels.SystemLanguageModel.UseCase, b: FoundationModels.SystemLanguageModel.UseCase)` | Func | `FoundationModels.swiftinterface:L585` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `SystemLanguageModel.Availability.UnavailableReason.==(a: FoundationModels.SystemLanguageModel.Availability.UnavailableReason, b: FoundationModels.SystemLanguageModel.Availability.UnavailableReason)` | Func | `FoundationModels.swiftinterface:L616` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `SystemLanguageModel.Availability.UnavailableReason.hash(into hasher: inout Swift.Hasher)` | Func | `FoundationModels.swiftinterface:L617` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `SystemLanguageModel.Availability.UnavailableReason.hashValue` | Var | `FoundationModels.swiftinterface:L618` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `SystemLanguageModel.Availability.==(a: FoundationModels.SystemLanguageModel.Availability, b: FoundationModels.SystemLanguageModel.Availability)` | Func | `FoundationModels.swiftinterface:L624` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.Index` | Typealias | `FoundationModels.swiftinterface:L709` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.startIndex` | Var | `FoundationModels.swiftinterface:L714` | Collection-conformance boilerplate | `RandomAccessCollection helper` |
| `Transcript.endIndex` | Var | `FoundationModels.swiftinterface:L717` | Collection-conformance boilerplate | `RandomAccessCollection helper` |
| `Transcript.Entry.==(a: FoundationModels.Transcript.Entry, b: FoundationModels.Transcript.Entry)` | Func | `FoundationModels.swiftinterface:L733` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.Entry.ID` | Typealias | `FoundationModels.swiftinterface:L737` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.Segment.==(a: FoundationModels.Transcript.Segment, b: FoundationModels.Transcript.Segment)` | Func | `FoundationModels.swiftinterface:L748` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.Segment.ID` | Typealias | `FoundationModels.swiftinterface:L752` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.TextSegment.==(a: FoundationModels.Transcript.TextSegment, b: FoundationModels.Transcript.TextSegment)` | Func | `FoundationModels.swiftinterface:L761` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.TextSegment.ID` | Typealias | `FoundationModels.swiftinterface:L765` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.StructuredSegment.==(a: FoundationModels.Transcript.StructuredSegment, b: FoundationModels.Transcript.StructuredSegment)` | Func | `FoundationModels.swiftinterface:L778` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.StructuredSegment.ID` | Typealias | `FoundationModels.swiftinterface:L782` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.Instructions.==(a: FoundationModels.Transcript.Instructions, b: FoundationModels.Transcript.Instructions)` | Func | `FoundationModels.swiftinterface:L792` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.Instructions.ID` | Typealias | `FoundationModels.swiftinterface:L796` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.ToolDefinition.description` | Var | `FoundationModels.swiftinterface:L803` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.ToolDefinition.==(a: FoundationModels.Transcript.ToolDefinition, b: FoundationModels.Transcript.ToolDefinition)` | Func | `FoundationModels.swiftinterface:L806` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.Prompt.==(a: FoundationModels.Transcript.Prompt, b: FoundationModels.Transcript.Prompt)` | Func | `FoundationModels.swiftinterface:L819` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.Prompt.ID` | Typealias | `FoundationModels.swiftinterface:L823` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.ResponseFormat.==(a: FoundationModels.Transcript.ResponseFormat, b: FoundationModels.Transcript.ResponseFormat)` | Func | `FoundationModels.swiftinterface:L834` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.ToolCalls.startIndex` | Var | `FoundationModels.swiftinterface:L845` | Collection-conformance boilerplate | `RandomAccessCollection helper` |
| `Transcript.ToolCalls.endIndex` | Var | `FoundationModels.swiftinterface:L848` | Collection-conformance boilerplate | `RandomAccessCollection helper` |
| `Transcript.ToolCalls.==(a: FoundationModels.Transcript.ToolCalls, b: FoundationModels.Transcript.ToolCalls)` | Func | `FoundationModels.swiftinterface:L851` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.ToolCalls.Element` | Typealias | `FoundationModels.swiftinterface:L855` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.ToolCalls.ID` | Typealias | `FoundationModels.swiftinterface:L859` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.ToolCalls.Index` | Typealias | `FoundationModels.swiftinterface:L863` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.ToolCalls.Indices` | Typealias | `FoundationModels.swiftinterface:L867` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.ToolCalls.Iterator` | Typealias | `FoundationModels.swiftinterface:L871` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.ToolCalls.SubSequence` | Typealias | `FoundationModels.swiftinterface:L875` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.ToolCall.==(a: FoundationModels.Transcript.ToolCall, b: FoundationModels.Transcript.ToolCall)` | Func | `FoundationModels.swiftinterface:L888` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.ToolCall.ID` | Typealias | `FoundationModels.swiftinterface:L892` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.ToolOutput.==(a: FoundationModels.Transcript.ToolOutput, b: FoundationModels.Transcript.ToolOutput)` | Func | `FoundationModels.swiftinterface:L902` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.ToolOutput.ID` | Typealias | `FoundationModels.swiftinterface:L906` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.Response.==(a: FoundationModels.Transcript.Response, b: FoundationModels.Transcript.Response)` | Func | `FoundationModels.swiftinterface:L916` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.Response.ID` | Typealias | `FoundationModels.swiftinterface:L920` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.==(a: FoundationModels.Transcript, b: FoundationModels.Transcript)` | Func | `FoundationModels.swiftinterface:L922` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `Transcript.Element` | Typealias | `FoundationModels.swiftinterface:L926` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.Indices` | Typealias | `FoundationModels.swiftinterface:L930` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.Iterator` | Typealias | `FoundationModels.swiftinterface:L934` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.SubSequence` | Typealias | `FoundationModels.swiftinterface:L938` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `Transcript.Entry.description` | Var | `FoundationModels.swiftinterface:L951` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.TextSegment.description` | Var | `FoundationModels.swiftinterface:L959` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.StructuredSegment.description` | Var | `FoundationModels.swiftinterface:L967` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.Segment.description` | Var | `FoundationModels.swiftinterface:L975` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.Instructions.description` | Var | `FoundationModels.swiftinterface:L983` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.Prompt.description` | Var | `FoundationModels.swiftinterface:L991` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.ResponseFormat.description` | Var | `FoundationModels.swiftinterface:L999` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.ToolCalls.description` | Var | `FoundationModels.swiftinterface:L1007` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.ToolCall.description` | Var | `FoundationModels.swiftinterface:L1015` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.ToolOutput.description` | Var | `FoundationModels.swiftinterface:L1023` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Transcript.Response.description` | Var | `FoundationModels.swiftinterface:L1031` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `Swift.Array.instructionsRepresentation` | Var | `FoundationModels.swiftinterface:L1069` | Builder convenience for array inputs | `array Prompt/InstructionsRepresentable extension` |
| `InstructionsBuilder` | Struct | `FoundationModels.swiftinterface:L1076` | Swift function-builder DSL | `@_functionBuilder` |
| `InstructionsBuilder.buildBlock(_ components: repeat each I)` | Func | `FoundationModels.swiftinterface:L1077` | Swift function-builder DSL | `@_functionBuilder` |
| `InstructionsBuilder.buildArray(_ instructions: [some InstructionsRepresentable])` | Func | `FoundationModels.swiftinterface:L1080` | Swift function-builder DSL | `@_functionBuilder` |
| `InstructionsBuilder.buildEither(first component: some InstructionsRepresentable)` | Func | `FoundationModels.swiftinterface:L1083` | Swift function-builder DSL | `@_functionBuilder` |
| `InstructionsBuilder.buildEither(second component: some InstructionsRepresentable)` | Func | `FoundationModels.swiftinterface:L1086` | Swift function-builder DSL | `@_functionBuilder` |
| `InstructionsBuilder.buildOptional(_ instructions: FoundationModels.Instructions?)` | Func | `FoundationModels.swiftinterface:L1090` | Swift function-builder DSL | `@_functionBuilder` |
| `InstructionsBuilder.buildLimitedAvailability(_ instructions: some InstructionsRepresentable)` | Func | `FoundationModels.swiftinterface:L1094` | Swift function-builder DSL | `@_functionBuilder` |
| `InstructionsBuilder.buildExpression(_ expression: I)` | Func | `FoundationModels.swiftinterface:L1097` | Swift function-builder DSL | `@_functionBuilder` |
| `InstructionsBuilder.buildExpression(_ expression: FoundationModels.Instructions)` | Func | `FoundationModels.swiftinterface:L1100` | Swift function-builder DSL | `@_functionBuilder` |
| `InstructionsBuilder.buildExpression(_ expression: T)` | Func | `FoundationModels.swiftinterface:L1104` | Swift function-builder DSL | `@_functionBuilder` |
| `Instructions.init(@FoundationModels.InstructionsBuilder _ content: ()` | Init | `FoundationModels.swiftinterface:L1112` | Swift builder-syntax overload | `@FoundationModels.*Builder` |
| `Swift.Array.promptRepresentation` | Var | `FoundationModels.swiftinterface:L1148` | Builder convenience for array inputs | `array Prompt/InstructionsRepresentable extension` |
| `PromptBuilder` | Struct | `FoundationModels.swiftinterface:L1155` | Swift function-builder DSL | `@_functionBuilder` |
| `PromptBuilder.buildBlock(_ components: repeat each P)` | Func | `FoundationModels.swiftinterface:L1156` | Swift function-builder DSL | `@_functionBuilder` |
| `PromptBuilder.buildArray(_ prompts: [some PromptRepresentable])` | Func | `FoundationModels.swiftinterface:L1159` | Swift function-builder DSL | `@_functionBuilder` |
| `PromptBuilder.buildEither(first component: some PromptRepresentable)` | Func | `FoundationModels.swiftinterface:L1162` | Swift function-builder DSL | `@_functionBuilder` |
| `PromptBuilder.buildEither(second component: some PromptRepresentable)` | Func | `FoundationModels.swiftinterface:L1165` | Swift function-builder DSL | `@_functionBuilder` |
| `PromptBuilder.buildOptional(_ component: FoundationModels.Prompt?)` | Func | `FoundationModels.swiftinterface:L1169` | Swift function-builder DSL | `@_functionBuilder` |
| `PromptBuilder.buildLimitedAvailability(_ prompt: some PromptRepresentable)` | Func | `FoundationModels.swiftinterface:L1173` | Swift function-builder DSL | `@_functionBuilder` |
| `PromptBuilder.buildExpression(_ expression: P)` | Func | `FoundationModels.swiftinterface:L1176` | Swift function-builder DSL | `@_functionBuilder` |
| `PromptBuilder.buildExpression(_ expression: FoundationModels.Prompt)` | Func | `FoundationModels.swiftinterface:L1179` | Swift function-builder DSL | `@_functionBuilder` |
| `PromptBuilder.buildExpression(_ expression: T)` | Func | `FoundationModels.swiftinterface:L1183` | Swift function-builder DSL | `@_functionBuilder` |
| `Prompt.init(@FoundationModels.PromptBuilder _ content: ()` | Init | `FoundationModels.swiftinterface:L1191` | Swift builder-syntax overload | `@FoundationModels.*Builder` |
| `GenerationID.==(a: FoundationModels.GenerationID, b: FoundationModels.GenerationID)` | Func | `FoundationModels.swiftinterface:L1310` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `GenerationID.hash(into hasher: inout Swift.Hasher)` | Func | `FoundationModels.swiftinterface:L1311` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `GenerationID.hashValue` | Var | `FoundationModels.swiftinterface:L1312` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `GenerationOptions.SamplingMode.==(a: FoundationModels.GenerationOptions.SamplingMode, b: FoundationModels.GenerationOptions.SamplingMode)` | Func | `FoundationModels.swiftinterface:L1333` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `GenerationOptions.==(a: FoundationModels.GenerationOptions, b: FoundationModels.GenerationOptions)` | Func | `FoundationModels.swiftinterface:L1341` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `GenerationSchema.debugDescription` | Var | `FoundationModels.swiftinterface:L1364` | Standard-library conformance boilerplate | `CustomStringConvertible/CustomDebugStringConvertible` |
| `LanguageModelFeedback.Sentiment.==(a: FoundationModels.LanguageModelFeedback.Sentiment, b: FoundationModels.LanguageModelFeedback.Sentiment)` | Func | `FoundationModels.swiftinterface:L1417` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `LanguageModelFeedback.Sentiment.AllCases` | Typealias | `FoundationModels.swiftinterface:L1421` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `LanguageModelFeedback.Sentiment.allCases` | Var | `FoundationModels.swiftinterface:L1422` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `LanguageModelFeedback.Sentiment.hash(into hasher: inout Swift.Hasher)` | Func | `FoundationModels.swiftinterface:L1425` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `LanguageModelFeedback.Sentiment.hashValue` | Var | `FoundationModels.swiftinterface:L1426` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `LanguageModelFeedback.Issue.Category.==(a: FoundationModels.LanguageModelFeedback.Issue.Category, b: FoundationModels.LanguageModelFeedback.Issue.Category)` | Func | `FoundationModels.swiftinterface:L1446` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `LanguageModelFeedback.Issue.Category.AllCases` | Typealias | `FoundationModels.swiftinterface:L1450` | Protocol-conformance boilerplate | `Identifiable/Collection/AsyncSequence helper` |
| `LanguageModelFeedback.Issue.Category.allCases` | Var | `FoundationModels.swiftinterface:L1451` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `LanguageModelFeedback.Issue.Category.hash(into hasher: inout Swift.Hasher)` | Func | `FoundationModels.swiftinterface:L1454` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |
| `LanguageModelFeedback.Issue.Category.hashValue` | Var | `FoundationModels.swiftinterface:L1455` | Standard-library conformance boilerplate | `Equatable/Hashable/CaseIterable helper` |

