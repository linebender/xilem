# Login

We want to support login in Placehero. As part of this, some refactors will be needed.

We now need some way to store data persistently, because we both need to store a login token, and to.
For now, we will assume that we are running this within the Xilem repository, and store it in `placehero/user_data/`, which will be `.gitignored`.
If we aren't in the Xilem repository, we just panic.
I believe that JSON is the right storage format here?
