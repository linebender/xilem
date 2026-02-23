# Your First Xilem App

You've got Xilem installed, you understand conceptually what it's trying to do, and now it's time to write some actual code. We're going to build the classic counter application: a window with a button that tracks how many times you've clicked it. This might sound trivial, but this small example contains every fundamental pattern you'll use in larger Xilem applications.

By the time you finish this chapter, you'll have written a working reactive application from scratch and you'll understand why each piece of code exists and how they all work together. More importantly, you'll start to feel how Xilem's reactive model actually works in practice, not just in theory.

## Creating Your Project

Let's start at the very beginning. Open your terminal and create a new Rust project:
```bash
cargo new xilem-counter
cd xilem-counter
```

This creates a new directory called `xilem-counter` with the standard Rust project structure. You'll have a `src/` directory containing `main.rs`, and a `Cargo.toml` file that describes your project.

Now we need to tell Cargo that our project depends on Xilem. Run this command:
```bash
cargo add xilem
```

If you open `Cargo.toml` now, you'll see that Cargo has added a line under `[dependencies]` that looks something like `xilem = "0.4.0"` (the version number might be different). This tells Rust's build system to download Xilem and all its dependencies when you build your project.

> **What's actually happening here?** When you run `cargo add xilem`, Cargo connects to crates.io (Rust's package registry), finds the latest version of Xilem, and adds it to your project's dependency list. The next time you build, Cargo will download not just Xilem but also all the libraries Xilem depends on, recursively. For Xilem, that's quite a few libraries because it builds on Masonry, which builds on Vello, Parley, and several other specialized crates. Don't be surprised if the first build takes a few minutes.

## Using the Development Version

Before we continue, there's an important detail about which version of Xilem we're using. The examples in this book use features from the latest development version of Xilem, which means we need to tell Cargo to use the code directly from the git repository instead of the published version on crates.io.

Open your `Cargo.toml` file and you'll see a line that looks like `xilem = "0.4.0"` under `[dependencies]`. Replace that entire line with:
```toml
xilem = { git = "https://github.com/linebender/xilem.git" }
```

This tells Cargo to download Xilem directly from the main branch of its GitHub repository, giving you access to the latest features and API improvements.

> **Why use the git version?** Xilem is evolving rapidly, with significant API improvements happening between releases. The published versions on crates.io can become outdated quickly as the team experiments with better designs. Using the git version means you're working with the same code the Xilem developers use daily, which makes it easier to follow discussions in the Zulip chat and ensures the examples in this book work correctly.
>
> The trade-off is that the git version is even more unstable than a published version would be. Breaking changes can happen at any time, sometimes even between the time you clone the repository and the next day. This is part of working with experimental software. If you need stability, Xilem isn't ready for you yet. But if you're comfortable with rapid change and want to be part of shaping the framework's direction, using the git version is the way to go.

After making this change, the next time you run `cargo build` or `cargo run`, Cargo will fetch Xilem from GitHub instead of crates.io.

## Writing the Simplest Possible Application

Open `src/main.rs` in your text editor. You'll see some default "Hello, world!" code that Cargo generated. Delete all of it and replace it with this:
```rust
use xilem::core::Edit;
use xilem::view::text_button;
use xilem::winit::error::EventLoopError;
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

fn app_logic(count: &mut u32) -> impl WidgetView<Edit<u32>> + use<> {
    text_button(format!("count: {}", count), |count: &mut u32| *count += 1)
}

fn main() -> Result<(), EventLoopError> {
    Xilem::new_simple(0, app_logic, WindowOptions::new("Counter"))
        .run_in(EventLoop::with_user_event())
}
```

Save the file and run your application:
```bash
cargo run
```

The first time you run this, Cargo will compile Xilem and all its dependencies. This will take a while. Go get a coffee. When it finishes, you'll see a window appear with a single button showing "count: 0". Click it and the number goes up. Click it again and it goes up more. That's it. You've built a reactive GUI application.

> **A note about compile times:** That initial compilation seems slow because Rust is building dozens of crates from source, many of them quite large. The good news is that this only happens once. Future builds will be much faster because Cargo caches everything. If you change just your `main.rs` file, rebuilding takes only a second or two.

Now let's understand exactly what you just wrote and why it works.

## The Imports: What You're Bringing Into Scope
```rust
use xilem::core::Edit;
use xilem::view::text_button;
use xilem::winit::error::EventLoopError;
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};
```

These lines bring several things from the Xilem crate into your code. **`Xilem`** is a struct that represents your entire application. It manages the window, runs the event loop, and orchestrates the reactive update cycle. **`EventLoop`** is what actually runs your application and handles system events. **`WindowOptions`** lets you configure things like the window title.

**`text_button`** is a function from Xilem's view module. Notice what you're importing: a function, not a struct or type. This is your first real hint that Xilem works differently from traditional GUI frameworks. Traditional frameworks give you widget classes that you instantiate and keep around. Xilem gives you functions that create lightweight descriptions of widgets, descriptions that live only briefly and get thrown away after each update cycle.

**`Edit<T>`** is a wrapper type that Xilem uses to track when state changes. You'll see it in the return type of your app logic function. **`WidgetView`** is a trait that describes something that can be turned into widgets on screen.

Think of it like the difference between building a house (traditional approach) versus drawing blueprints (Xilem's approach). When you call `text_button(...)`, you're not constructing the actual button widget. You're creating a blueprint that says "there should be a button here, and it should look like this." Xilem takes your blueprint and builds or updates the real widget as needed.

## Understanding Application State
```rust
fn app_logic(count: &mut u32) -> impl WidgetView<Edit<u32>> + use<> {
```

This function signature tells you something fundamental about how Xilem applications work. **Every Xilem application has state**, and that state lives in one central place. In our counter, the state is just a single number representing how many times the button has been clicked. In a real application, the state might be a complex struct with dozens of fields, but the principle is the same: your entire application's data lives in one place.

The parameter `count: &mut u32` gives your app logic function access to that state. The `&mut` is crucial. It means you're receiving a *mutable reference* to the state. Event handlers will be able to modify this state when users interact with your interface. Without the `mut`, your interface could display data but never change it.

The return type `impl WidgetView<Edit<u32>> + use<>` says "this function returns something that implements the `WidgetView` trait, and that view works with state wrapped in `Edit<u32>`." The `Edit` wrapper is how Xilem tracks changes to your state. The `impl` keyword lets you avoid writing out the exact type, which is convenient because view types can get complicated and verbose as your interface grows. The `+ use<>` at the end is a Rust feature that tells the compiler not to capture any lifetimes implicitly in this return type, which is necessary for Xilem's reactive system to work correctly.

> **Why does the return type mention the state type?** Xilem needs to know what type of state your views expect so it can check at compile time that everything matches up. If you try to return a view that expects a `String` from a function that receives a `u32`, the compiler will catch that mistake before you ever run your program. This is Rust's type system helping you write correct code.

## Creating the View Tree
```rust
    text_button(format!("count: {}", count), |count: &mut u32| *count += 1)
```

This single line creates your entire user interface. Let's break it down into its parts.

**`text_button(...)`** is a function call that creates a button view with text inside it. It's a convenience function that combines a button with a label. It takes two arguments: the button's label text, and what happens when someone clicks it.

**`format!("count: {}", count)`** creates the label text. The `format!` macro works like `println!` but returns a `String` instead of printing to the console. The `{}` is a placeholder that gets replaced with the value of `count`. When your state is `0`, this produces the string "count: 0". When your state is `5`, this produces "count: 5". Every time Xilem calls your app logic function with a different count value, this expression produces a new label string.

Here's something important to understand: you're not updating the button's label when the count changes. You're describing what the label *should be* for any given count. This might seem like a subtle distinction, but it's actually profound. In a traditional GUI framework, you'd write code that says "when the count changes, find the label widget and set its text to the new value." In Xilem, you write code that says "the label is always the count formatted as text." The framework figures out when it needs updating.

**`|count: &mut u32| *count += 1`** is a closure, an anonymous function that gets called when the user clicks the button. Closures in Rust are written with vertical bars around the parameters. This closure receives a mutable reference to the state, and it increments that state by one. Notice that we explicitly annotate the parameter type as `&mut u32`. While Rust can often infer closure parameter types, in this case the explicit annotation helps the compiler understand exactly what types are involved in the reactive system.

The `*` before `count` is Rust's dereference operator. Since `count` is a reference to a `u32`, not a `u32` itself, we need to dereference it to access the actual number so we can add to it. Think of it like following a pointer to get to the actual data.

When you click the button, here's what happens in sequence: the click event fires, Xilem calls your closure with `&mut count`, the closure increments the count, and then Xilem calls your `app_logic` function again with the new count value. Your `app_logic` function returns a new button view with an updated label. Xilem compares this new view against the old one, sees that only the label text changed, and updates just that text on the screen. The button itself doesn't get destroyed and recreated; only its label updates.

## Starting the Application
```rust
fn main() -> Result<(), EventLoopError> {
    Xilem::new_simple(0, app_logic, WindowOptions::new("Counter"))
        .run_in(EventLoop::with_user_event())
}
```

This is where everything begins. Let's look at each piece.

**`Xilem::new_simple(0, app_logic, WindowOptions::new("Counter"))`** creates a new Xilem application. It needs three things to get started: the initial state, the app logic function that knows how to turn state into a user interface, and options for the window.

The first argument, `0`, is your application's starting state. Before any button clicks, before any user interactions, this is what the count will be. When your application first starts, Xilem will call `app_logic(0)` to build the initial interface.

The second argument is your `app_logic` function. Notice you're passing the function itself, not calling it. In Rust terms, you're passing a function pointer. Xilem will call this function whenever it needs to rebuild the interface, which happens initially and then again every time state changes.

The third argument creates window options with the title "Counter". This is what appears in your window's title bar.

**`.run_in(EventLoop::with_user_event())`** starts the application's event loop and doesn't return until the user closes the window. The `EventLoop::with_user_event()` creates an event loop that can handle both system events (like mouse clicks and key presses) and custom user events. While it's running, Xilem is continuously listening for events: mouse movements, clicks, key presses, window resizes. When something happens that might affect your application, Xilem responds appropriately. For button clicks specifically, it runs the event handler, which modifies state, which triggers a call to `app_logic` with the new state, which returns a new view tree, which Xilem reconciles against the old tree to figure out what actually needs updating on screen.

This might all happen in a few milliseconds, fast enough that it feels instant to users, but the cycle is always the same: event → state change → app logic → new view tree → reconciliation → UI update.

## Making It More Interesting: Two Buttons

Your counter works, but it's pretty limited. Let's add a reset button so you can start counting over. Here's the new version:
```rust
use xilem::core::Edit;
use xilem::view::{flex_col, text_button};
use xilem::winit::error::EventLoopError;
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

fn app_logic(count: &mut u32) -> impl WidgetView<Edit<u32>> + use<> {
    flex_col((
        text_button(format!("count: {}", count), |count: &mut u32| *count += 1),
        text_button("reset", |count: &mut u32| *count = 0),
    ))
}

fn main() -> Result<(), EventLoopError> {
    Xilem::new_simple(0, app_logic, WindowOptions::new("Counter"))
        .run_in(EventLoop::with_user_event())
}
```

Run this and you'll see two buttons stacked vertically. The top one still counts up, and the new one resets the counter to zero. Let's understand what changed.

**`flex_col`** is a layout container that arranges its children vertically. You imported it alongside `text_button` at the top of the file. The "flex" part means it uses flexible sizing: children can grow or shrink based on available space. For our simple example, that doesn't matter much, but for more complex layouts, flexible containers are how you create interfaces that adapt gracefully to different window sizes.

The argument to `flex_col` is a tuple: `(button1, button2)`. In Rust, tuples are written with parentheses and can contain different types. Xilem understands tuples of views as lists of children to display inside the container. The order matters: the first button will appear at the top, the second below it.

**The reset button** shows a simpler pattern than the counter button. Its label is just the string "reset", not a formatted string that changes with state. Labels don't have to be dynamic; they can be static text when that's all you need.

The reset button's event handler is `|count: &mut u32| *count = 0`, which sets the count directly to zero instead of incrementing it. When you click this button, the state becomes `0`, Xilem calls `app_logic(0)`, and both buttons update to reflect the new state. The counter button goes back to showing "count: 0", and everything is back to the beginning.

> **Notice what you didn't have to do.** You didn't write any code to coordinate these two buttons. You didn't tell the counter button "hey, when the reset button gets clicked, you need to update your label." You didn't register the reset button as a listener for count changes. You just described what each button should display and what it should do when clicked. Xilem keeps everything in sync automatically because everything flows from the same source of truth: the count variable.

## Working with Structured State

Our counter uses a single integer for state, which is fine for this example but obviously oversimplified for real applications. Let's make it more realistic by tracking both the current count and the total number of clicks across all resets.

Replace your code with this version:
```rust
use xilem::core::Edit;
use xilem::view::{flex_col, text_button};
use xilem::winit::error::EventLoopError;
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

struct AppData {
    count: u32,
    total_clicks: u32,
}

fn app_logic(data: &mut AppData) -> impl WidgetView<Edit<AppData>> + use<> {
    flex_col((
        text_button(format!("count: {}", data.count), |data: &mut AppData| {
            data.count += 1;
            data.total_clicks += 1;
        }),
        text_button("reset", |data: &mut AppData| data.count = 0),
        text_button(
            format!("total clicks: {}", data.total_clicks),
            |_data: &mut AppData| {
                // This button just displays information; it doesn't do anything when clicked
            },
        ),
    ))
}

fn main() -> Result<(), EventLoopError> {
    let initial_state = AppData {
        count: 0,
        total_clicks: 0,
    };
    Xilem::new_simple(initial_state, app_logic, WindowOptions::new("Counter"))
        .run_in(EventLoop::with_user_event())
}
```

Now your application shows three buttons. The first increments both the count and the total. The second resets the count but leaves the total unchanged. The third just displays the total and doesn't do anything when clicked.

**The state is now a struct** called `AppData` with two fields. This is how you handle more complex application state in Xilem. As your application grows, you might have a struct with dozens of fields, maybe nested structs inside it, collections like `Vec` and `HashMap`, whatever data structure makes sense for your application. The pattern stays the same: one central state struct that contains everything your application needs to remember.

**The increment button's event handler** now has a body with multiple statements instead of just a single expression. Notice the curly braces that weren't there before. This closure modifies both fields of the state: it increments the count like before, but it also increments the total. Each click is counted in two ways.

**The reset button** only modifies `data.count`, leaving `data.total_clicks` untouched. This is why tracking totals across resets works: the reset action is selective about what it changes.

**The third button** shows an interesting pattern. Its event handler is `|_data: &mut AppData| { }`, a closure that receives the data but doesn't use it (the underscore prefix tells Rust "I know I'm not using this parameter, don't warn me about it") and has an empty body. This button is purely informational. It displays data but doesn't respond to clicks. You could remove the event handler entirely and pass `|_| {}` instead (no parameters at all), but keeping the data parameter makes the types work out more smoothly.

Run this version and experiment with it. Click the counter several times, hit reset, click the counter more, hit reset again. Watch how the count resets but the total keeps growing. This demonstrates a key advantage of centralized state: you can easily track different aspects of your application's history because it's all right there in one place.

## Understanding the Update Cycle

Let's trace exactly what happens during one complete interaction, from click to screen update. This will help you understand how Xilem's reactive model works in practice.

**1. Initial state:** Your application starts with `AppData { count: 0, total_clicks: 0 }`. Xilem calls `app_logic` with this state and gets back a view tree describing three buttons. It builds actual widgets from these views and displays them.

**2. User clicks the counter button:** The click generates an event. Xilem sees that this particular button widget has an event handler associated with it (the closure you provided).

**3. Event handler runs:** Xilem calls your closure with `&mut AppData`. The closure modifies the state to `AppData { count: 1, total_clicks: 1 }`.

**4. App logic runs again:** Xilem calls `app_logic` with the modified state. Your function executes with `count` and `total_clicks` both being `1`, so `format!` produces different strings than before.

**5. New view tree created:** `app_logic` returns a new view tree. It still has a `flex_col` with three buttons, but the first button's label is now "count: 1" and the third button's label is "total clicks: 1".

**6. Reconciliation happens:** Xilem compares the new view tree against the old one. The structure is identical (still three buttons in a column), but two of the button labels changed. Xilem notes these differences.

**7. UI updates:** Xilem updates just the two label texts on the existing button widgets. It doesn't destroy and recreate the buttons. It doesn't rebuild the layout. It changes only what actually changed: two strings of text.

**8. Application waits:** The screen now shows the updated interface, and Xilem goes back to waiting for the next event.

This whole cycle typically completes in a few milliseconds. From a user's perspective, clicking the button immediately updates the display. But understanding these steps helps you see why Xilem works the way it does: the event handler modifies state, state flows into app logic, app logic produces views, views get reconciled, and the UI updates minimally.

## What Makes This Reactive

The term "reactive" gets thrown around a lot in GUI frameworks, so let's be precise about what it means in Xilem's context.

**You declare relationships, not procedures.** In a traditional imperative GUI framework, you'd write code that says "when the user clicks this button, execute this sequence of steps: increment the counter, find the label widget, convert the new count to a string, set the label's text property to that string." You're giving step-by-step instructions.

In Xilem, you write code that says "the label's text is always the count formatted as a string." You're declaring a relationship between the state and the interface. When state changes for any reason, the relationship is automatically maintained because Xilem re-evaluates it.

**State changes propagate automatically.** When you modify state in an event handler, you don't have to manually update every part of the UI that might be affected. You just change the state, and Xilem calls your app logic function to get a fresh view tree that reflects the new state. Any part of the interface that depends on the changed data gets updated automatically because the view tree describes it as depending on that data.

**The framework calculates minimal updates.** You don't have to think about which specific widgets need updating. You rebuild the entire view tree on every state change (conceptually, anyway; memoization can optimize this), and Xilem figures out what actually needs to change on screen. This might seem wasteful, but it's not; comparing lightweight view descriptions is cheap, and rebuilding the actual heavy widgets is expensive, so Xilem avoids the expensive part whenever possible.

These three properties together, declaring relationships plus automatic propagation plus minimal updates, are what make Xilem a reactive framework. The model scales from simple counters to complex applications with thousands of widgets and megabytes of state because the pattern never changes: describe what the interface should be, modify state when things happen, and let the framework handle the rest.

## Try It Yourself

Before moving on to the next chapter, try extending this counter in different ways. Here are some ideas:

Add a decrement button that reduces the count by one. Add a multiply button that doubles the current count. Add a text label above the counter button that says something different based on whether the count is even or odd. Change the layout from `flex_col` to `flex_row` to see the buttons arranged horizontally instead of vertically.

Experiment until you feel comfortable with the pattern: state lives in one place, app logic transforms state into views, event handlers modify state. When you can write these without thinking too hard about the mechanics, you're ready to learn about the deeper concepts that make it all work.

The next chapter dives into components, the view tree structure, and how reconciliation actually works behind the scenes. You'll learn how to compose larger interfaces from smaller pieces and how Xilem efficiently updates complex UIs.