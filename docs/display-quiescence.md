# Display quiescence contract

Status: portable contract and Linux adapters implemented under T1050. Production
USB activity composition remains T1051; physical acceptance remains T914.

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
