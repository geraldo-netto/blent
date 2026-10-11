# T960: explicit coroutine opt-in

Declare ExperimentalCoroutinesApi on the existing native exercise method using
getCompleted. Runtime logic and assertions are unchanged. Normal Android test
fixture compilation passes without its previous opt-in warnings; all four methods
remain <=9. Declaration-only benchmark change needs no artificial runtime test.
