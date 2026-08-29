# Rubyn Harness

Rubyn Harness coordinates human decisions and agent work across Ruby and Rails projects while keeping every consequential change inspectable.

## Language

**Conversation**:
A durable project-scoped dialogue with Rubyn that a person may rename, pin, or archive. Finishing or stopping an engine turn does not close the Conversation; a follow-up resumes it in the same retained worktree. A Conversation is not execution accounting, even when it has an inspectable worktree.
_Avoid_: Run, session, thread

**Background Run**:
Agent execution launched for a Board Task or Wayfinder Ticket. Only Background Runs contribute to run activity and history.
_Avoid_: Conversation, chat

**Wayfinder Map**:
A project-scoped model of a destination and the uncertain, dependent work required to reach it.
_Avoid_: Plan, roadmap, board

**Destination**:
The observable future state that determines when a Wayfinder Map has succeeded.
_Avoid_: Goal, vision

**Wayfinder Ticket**:
A bounded unit of uncertainty or work within a Wayfinder Map, typed as Grill, Research, Prototype, Code, or User Action.
_Avoid_: Card, issue, task

**Frontier**:
The set of unresolved Wayfinder Tickets whose dependencies are settled and which may be acted on now.
_Avoid_: Queue, backlog

**Fog**:
Uncertainty that prevents the Map from identifying or safely acting on its next frontier.
_Avoid_: Risk, unknowns

**Grill Round**:
Up to three independent human questions presented together to settle a Grill Ticket.
_Avoid_: Survey, interview step

**Graph Delta**:
A proposed set of additions, dependency changes, or retirements to a Wayfinder Map that becomes effective only after approval.
_Avoid_: Update, patch

**Resolution**:
The accepted decision or evidence that settles a Wayfinder Ticket.
_Avoid_: Answer, result

**User Action**:
A Wayfinder Ticket representing work that only a person can complete and whose resolution requires a result note.
_Avoid_: Approval, blocker

**Board Task**:
The execution record created from an unblocked Code Ticket. Wayfinder owns its brief and dependencies; the task board owns its execution through integration.
_Avoid_: Code Ticket

**Agent Profile**:
A named, project-scoped set of standing instructions that Rubyn adopts for a kind of work, composed of a Mission, Starting Context, Working Method, Finish Line, and Guardrails. It shapes Rubyn's behavior; it is not a separate actor, running process, or conversation.
_Avoid_: Run, Conversation, bot

**Column Policy**:
The Agent Profile selected when a Board Task enters a workflow column. It chooses Rubyn's standing instructions but never spends model tokens by itself.
_Avoid_: Automation, hook

**Brief Version**:
The immutable version of a Wayfinder Ticket's objective, information, outcome, and dependencies seen by a launched Background Run.
_Avoid_: Revision, prompt version
