# Display quiescence contract

Status: portable contract and Linux adapters implemented under T1050; production
USB activity composition implemented under T1051. Joined acceptance remains
T1016/T1009; physical acceptance remains T914.

One display runtime owns one controller state. Its capture and selector producers
are bound before startup, with a fixed capacity of one or two. Controller clones
address the same owner; only the current display assignment may issue commands.
Composition must hand the original controller and its producers together through
startup. Unavailable backends expose no controller. Read-only sources, permits and
fences retain observations, never controller ownership or acquisition authority.

An open request admits work with that original permit. Pause invalidates that
permit and prevents further admission. Repeated pause retains the same request.
Each original producer publishes its own matching receipt only after verifying
its native obligations: original creators, reader/encoder retirement and either
the original retained-helper writer pause ACK or terminated helper/stdout
retirement. Selector receipt requires its complete original native trial group,
including work created after parent cancellation. A task result, timeout, desired
state, reference count or unverified receipt is insufficient evidence.

Resume requires the exact current stop and all original producer receipts. It
creates a fresh permit; old observations cannot revive through coalesced updates.
Foreign/stale requests and closed producers/controllers deny resume. Unknown or
stalled obligations remain owned and funded. Native ownership and its existing
cleanup sponsors remain authoritative; the handshake never releases them.

Storage holds one current request, at most two receipt publications and bounded
read views retained by the caller. Each assignment must retain at most its current
fence. Identity objects remain alive while borrowed, preventing premature identity
reuse. No producer/controller retention cycle is permitted. Eventual progress
requires actual original cleanup, a live producer and continued owner scheduling;
owner loss fails closed and needs the enclosing runtime's actual retirement.

The permanent model exhausts 1,024 five-command traces over pause, individual
receipts and resume. Native fixtures link receipts to held cleanup, late creators
and exact helper ACKs; a deliberately premature adapter receipt is rejected by
the oracle. These finite checks and hosted native traces complement each other;
they do not prove unrestricted kernel/device safety or physical presentation.

## Renewable USB composition

One selected assignment owns its capacity ticket, original renewable display
client and bounded activity binding. The native adapter supplies its original
runtime controller and actual whole-runtime completion together, plus a borrowed
view of the shared machine source. No new machine source or auxiliary resource
owner is created. Unsupported native compositions reject the binding explicitly;
absence of machine observation retains the existing unsupported-source policy.

Initial handoff pauses original display work before exposing a USB registration.
An unavailable observation or discontinuous work stamp revokes only the original
display route and closes native admission. The selected ticket, listeners, geometry
and other configuration remain owned. Original broker retirement and all original
native receipts must settle before another attempt; retry uses the existing
absolute deadline. Fresh credentials are prepared for the current authenticated
discovery candidate. Native work opens only after that fresh route reports Active,
so existing capture supervision must start a fresh codec generation/keyframe.
The coordinator reconciles both before and after broker polling, fencing late
startup and coalesced stop/resume. Machine publication wakes the daemon directly;
its bounded tick remains a fallback. Closed observation does not create a spin.

Prompt controller commands serialize against attachment replacement using the
original live or revoked lease. A stale assignment cannot pause, resume or revoke
a replacement. If its original scoped fence cannot be established, capacity stays
owned until actual original whole-runtime completion. This conservative fallback
can retain capacity until daemon shutdown; neither elapsed time nor a logically
closed shared assignment is retirement evidence. Current native obligations keep
their existing cleanup sponsors. Eventual reuse assumes original cleanup and
continued scheduling, or eventual actual enclosing-runtime retirement after loss.

At most four selected assignments retain one current native fence and one work
observation each. Borrowed machine/controller views contain no capture allocator,
media buffer or route grant. Closing/dropping requests original display and runtime
cleanup; native owners and descriptor-bound clients retain actual obligations on
partial failure, cancellation, owner loss and exhaustion. Auxiliary media has its
own owners and grants; display activity never commands their close/retirement.
