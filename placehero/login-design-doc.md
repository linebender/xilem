# Login

We want to support login in Placehero. As part of this, some refactors will be needed.

We now need some way to store data persistently, because we both need to store a login token, and to.
For now, we will assume that we are running this within the Xilem repository, and store it in `placehero/user_data/`, which will be `.gitignored`.
If we aren't in the Xilem repository, we just panic.
I believe that JSON is the right storage format here?

As part of the login project, we're taking another look at our data design.
Therefore, let's do some thinking about what we need to support.

## Transitions needed

Full back stack:
Out of scope for now.
(In particular, our current virtual scrolling design makes this hard - Imperative actions strikes again)

Partial Back Stack (i.e. browse, dig into one message, back out again):

- Probably worth supporting. Pretty trivial with an `IndexedStack`
- Similar to current model, but maintaining some state better
