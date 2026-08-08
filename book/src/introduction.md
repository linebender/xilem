# Introduction

> **Note:** This book is currently under construction. Many chapters exist as placeholders waiting for content. We're building this documentation alongside the framework itself, and contributions are welcome.

Building graphical user interfaces in Rust has historically meant choosing between writing verbose imperative code that manually orchestrates every UI change, or reaching for web technologies wrapped in something like Tauri. Xilem offers a different path: a reactive framework built from the ground up for Rust, where you describe what your interface should look like and the framework figures out how to get there efficiently.

If you've worked with React, SwiftUI, or Elm, you'll recognize the core idea. Your application has state, and you write functions that transform that state into a description of your interface. When the state changes, Xilem automatically calculates the minimal set of updates needed to bring your UI in sync. This declarative approach means you spend less time thinking about how to update individual widgets and more time thinking about what your application should do.

This book will teach you how to harness this reactive model to build native desktop applications in Rust. We'll start small with a simple example and build up to the patterns and techniques you need to create real applications.

## A Word About Experimental Software

> **Important:** Xilem is at version 0.4 and highly experimental. The API is evolving rapidly, and breaking changes happen frequently - even in minor releases.
>
> As the Xilem team recently noted: "We've decided to experiment with some radical changes to the Xilem API, and until we've decided which changes we want to keep, it's hard to predict what the roadmap will be."

What does this mean in practice? The code you write today will likely need adjustments as Xilem evolves. Features you rely on might change their API or even disappear entirely if the team discovers a better approach. Some things that should work might not yet, and you'll occasionally run into rough edges that haven't been polished smooth.

This isn't a bug in Xilem's development process - it's the natural state of a framework that's being designed in public, with active experimentation happening at the architectural level. The Xilem team is trying to find the right abstractions for reactive UI in Rust, and that sometimes means throwing out code and starting over when a better idea emerges. If you need production-ready stability today, Xilem isn't there yet. But if you're interested in seeing how a modern UI framework takes shape and want to be part of that process, you're in exactly the right place.

## What This Book Teaches

We'll start your journey by showing you a complete working Xilem application and walking through what each part does. This hands-on beginning gets you writing code immediately while introducing the mental models you need to understand how Xilem works.

From there, we'll explore the fundamental concepts that make Xilem tick. You'll learn how components encapsulate both state and the logic that renders it, how the view tree efficiently represents your UI, and how reconciliation figures out what actually needs to change when your state updates. We'll cover event handling so you can respond to user interactions, and then move into more advanced patterns for composing complex interfaces from simple, reusable pieces.

Along the way, you'll find a glossary explaining terms that come up repeatedly in Xilem discussions, and we'll have honest conversations about what's working well in the framework and what's still being figured out. Some of these discussions will end with "we're not entirely sure this is the right approach yet" - and that's okay. Understanding what's uncertain helps you make better decisions about when and how to use Xilem.

## What You Need to Know

You should be comfortable with Rust fundamentals: ownership, borrowing, and traits in particular. If those concepts still feel shaky, spend some time with [The Rust Book](https://doc.rust-lang.org/book/) first, at least through the chapters on these topics. Xilem leverages Rust's type system heavily, and you'll have a much better time if you're not simultaneously wrestling with the language and the framework.

You don't need prior experience building graphical interfaces, though if you have worked with traditional imperative GUI frameworks like GTK or Qt, you'll find it interesting to see how Xilem's declarative approach changes the way you think about UI updates. Similarly, if you're coming from web frameworks like React, you'll recognize the reactive patterns even though the implementation details differ.

## Getting Started

The next chapter drops you straight into code. You'll see a complete Xilem application, understand what each line does, and run it on your own machine. From that concrete starting point, the rest of the book builds outward, giving you progressively deeper understanding of how and why Xilem works the way it does.

Ready? Let's build something.