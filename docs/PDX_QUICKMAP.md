# PDX_QUICKMAP

## Mental Model
PDX is not POSIX IPC.
It is the normal communication path between protection domains.

## Safe Pattern
Producer:
- use shared constants
- pack scalar args only
- call target slot
- do not pass cross-PD raw pointers

Consumer:
- listen on correct slot
- match opcode
- validate args
- mutate only owned state
- reply when protocol expects reply

## Required Checks
For any PDX bug, verify:
1. sender slot
2. receiver listen slot
3. opcode constant equality
4. capability grant exists
5. arg packing matches decode
6. receiver applies state
7. no pointer args unless capability-backed

## Forbidden
- duplicated magic opcodes
- client-supplied caller identity
- cross-PD raw pointer dereference
- fallback Linux/POSIX semantics
