# T955: cohesive management boundary fixtures

Separate camera/audio profile boundaries from key/credential/capability checks.
Retain every original value, encode/rejection assertion, credential immutability
check, long-key acceptance, cross-resource profile rejection and bounded wire
mutation test. Original T850 test remains; the new T955 test is part of the normal
suite. All fixtures pass without skips; all eight functions score <=9.
Only permanent test organization changed; production behavior is unchanged.
