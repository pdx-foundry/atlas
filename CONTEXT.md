# PDX Atlas

PDX Atlas describes engine-defined Stellaris scripting rules and their supporting evidence, independently of any vanilla or mod content catalogue or project state.

## Language

**PDX Native**:
The engine API that supplies static compiler facts and small live checks, and owns platform and build differences. Atlas is its only consumer.
_Avoid_: Native foundation (former name)

**PDX Atlas**:
The rule-knowledge system that derives supported game rules from engine evidence and supplies them to offline consumers. Its rule extraction and rule model are platform-independent; PDX Native owns native target operations.

**Game fact**:
A statement about the game supported by evidence within stated conditions. Its scope does not extend beyond what that evidence establishes.

**Rule**:
An evidence-supported statement about game behavior, including constraints, processing, and fallback behavior. A rule is independent of any particular project's content, even when applying it requires values from that content; it does not prescribe authoring advice.

**Conditional rule**:
A rule that states the conditions under which the described behavior applies.

**Project input**:
Consumer-owned authored content and available vanilla or mod dependency definitions to which the consumer applies Atlas rules. Project inputs supply values to check and references to resolve; they are neither Atlas rules nor authority for them.

**Structural constraint**:
An evidence-supported rule describing a script value's form or reference category under stated conditions. The constraint does not prescribe a consumer's diagnostic severity.

**Conditional constraint**:
A machine-readable constraint whose applicability depends on stated input conditions, applied by the consumer to its project inputs. It does not simulate game execution or determine whether a runtime trigger succeeds.

**Rule documentation**:
Explanations of static facts that change what a compiler accepts, rejects, types or completes. They state the evidence and limits of shapes, references, conversion rules, field conditions and entry scopes.

**Parser storage**:
The value retained after the game reads an authored input. Storage alone does not establish successful validation or the value used during execution.

**Engine validation**:
The game's checks and diagnostics for an input at a stated processing stage. Continued execution does not by itself establish that an input passed these checks.

**Runtime outcome**:
An evaluated value or action when the game uses an input. Runtime outcomes are out of scope under the simplification decision of 2026-10-02.

**Authoring recommendation**:
Advice about what an author should write, owned by a consumer such as the SDK. It is separate from facts about what the game stores, validates, or does.

**Source claim**:
A statement made by an identified source. Its presence in documentation or CWTools config does not by itself establish that the game behaves as stated.

**Evidence**:
Traced source material, analysis, or observations that support or challenge an exact claim within stated conditions. Evidence for what a source says is distinct from evidence for what the game does.

**Content observation**:
A value or structure found in the examined game content. An observation does not by itself establish an exhaustive set of permitted values or structures.

**Scope availability**:
Whether a scope is usable in a running game. This is runtime meaning and is out of scope. The compiler instead needs the declared scope type at entry.

**Scope identity**:
The relation of `this`, `root`, `from` and `prev` established from call sites. Static self-link assumptions and their checks remain explicit; Atlas does not promise the identity of a live game object.

**Scope contract**:
The declared entry scope (`this`) and the static call-site relationships for `root`, `from` and `prev`. Unresolved relationships remain gaps. A check of one entry path supports only that path.

**Unresolved property**:
A question about the game whose answer has not yet been established. Missing extraction or verification does not establish that a property requires permanent manual maintenance.

**Unknown**:
The answer to a game-property question is not established. This does not imply that the engine permits or forbids the behavior.

**Unsupported**:
Outside the declared consumer support boundary. Some facts about an unsupported feature may still be known.

**Contradicted claim**:
A claim challenged by applicable evidence. The challenge does not by itself establish an alternative answer.

**Untested**:
Not examined by a specified test or validation method. A property can be untested by that method while supported by other evidence.

**Extraction target**:
The game build and platform from which Atlas produces engine evidence. It is distinct from the platform on which an offline consumer runs.

**Rule snapshot**:
An immutable, versioned collection of Atlas rules, documentation, coverage gaps, and evidence references for explicitly supported game versions. A correction is a newer snapshot version that can target the same game version.

**Rule identity**:
A stable identifier for a particular rule subject and property across snapshots. The snapshot identifies the published answer; correcting that answer does not by itself create a new rule identity.

**Snapshot applicability**:
The game targets and conditions for which a snapshot's rule claims are supported by evidence. The ability to read a snapshot on a platform does not establish its applicability to that platform's game executable.

**Evidence conflict**:
An apparent inconsistency between evidence about the same engine property under the same conditions. Different processing stages, execution paths, or builds can explain an apparent conflict without either observation being wrong.

**Method validation**:
Evidence that an extraction or observation method measures the property it claims to measure within stated limits. Validation of a method does not establish properties that the method has not examined.

**Coverage obligation**:
An individually assessable game property within stated conditions needed to support a coverage slice. Repeated uses of a shared fact do not create additional established facts, and unsplit prose is not an established atomic property.

**Coverage slice**:
A bounded authoring workflow and the game knowledge needed to support it, including its required relationships. Its boundary is not limited to information present in config.

**Coverage gap**:
Missing knowledge within a stated coverage slice. A gap that prevents a promised consumer check blocks the claim that the slice is complete, even when other facts in it are supported.
