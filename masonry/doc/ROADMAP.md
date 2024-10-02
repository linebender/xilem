# TODO - MVP

- [X] Clean up imports
- [X] Add impl Deref for WidgetRef
- [X] Add downcast to WidgetRef and WidgetView
- [X] Add WidgetAdded test
- [X] Rename ContextState to GlobalPassCtx
- [X] Sort WidgetState fields
- [X] Remove ExtendDrain

- [X] Rewrite tests
 - [X] Create coverage profile
 - [X] Look at coverage of every file, write a list of target coverage
 - [X] Add timers to harness

- [X] Add action queue
- [X] Re-add ExtEvent + AppDelegate

- [X] Use WidgetView everywhere
 - [X] Button
 - [X] Checkbox
 - [X] Flex
 - [X] Image
 - [X] Label
 - [X] SizedBox
 - [X] Spinner
 - [X] Portal

- [X] Implement TextBox
 - [X] Implement Scroll
 - [X] Add text/IME to harness
 - [X] Port TextBox
 - [X] Write unit tests

- [X] Refactor ScrollComponent / paint_rect / paint_insets
 - [X] Set paint_rect automatically
 - [X] Check that paint_rect has children paint_rect
 - [X] Disambiguate concept of hidden/stashed widgets

- [X] Refactor different passes
 - [X] Write a WidgetCtx trait
 - [X] Add a `WidgetPod::as_mut(ctx: &impl WidgetCtx)` method

- [X] Fix WidgetView type
 - [X] Rename WidgetView to WidgetMut
 - [X] Split WidgetRef into separate file
 - [X] Create StoreInWidgetMut from (Widget, WidgetCtx)
 - [X] Create `declare_widget!()` macro to implement boilerplate.

- [X] Write examples
 - [X] To-do List
 - [X] Fix blocking_function example
 - [X] Copy examples from druid

- [X] Fix doc tests

- [X] Split off from Druid repo
- [X] Rename to Masonry
- [X] Write remaining tasks in Github
- [X] Rewrite License bits
- [X] Write ARCHITECTURE.md
- [X] Write CONTRIBUTING.md
- [X] Write README.md and doc root
- [X] Publish

# TODO - Next steps

- [X] Remove Env type and Data trait (<https://github.com/linebender/xilem/issues/373>)
- [ ] Re-add Dialog feature (<https://github.com/linebender/xilem/issues/356>)
- [ ] Switch to using Vello and Glazier (<https://github.com/linebender/xilem/issues/357>)
- [ ] Refactor TextLayout (<https://github.com/linebender/xilem/issues/358>)

- [ ] Rework Widget trait (<https://github.com/linebender/xilem/issues/355>)

- [ ] Write Widget Inspector

- [ ] Port Panoramix to Masonry
- [ ] Port Xilem to Masonry

# TODO - Long term

- [ ] Brainstorm names for AppDelegate, Harness methods, Action, ExtEvent

- [ ] Move debug logger to use tracing infrastructure

- [ ] Refactor Mouse Events and hot status changes

- [ ] Remove `impl Widget for Box<dyn Widget>`

- [ ] Handle request-layout-in-layout corner case

- [ ] Add logo

- [ ] Create project board

- [ ] Refactor WidgetState
 - [ ] Add WidgetTmpState
 - [ ] Extract WidgetState fields to WidgetTmpState + global state

- [ ] Rework place_child
- [ ] Fix invalidation when computing layout

- [ ] Write analysis of different layout footguns
 - Flex interacting with viewport
 - Does Tennent's Correspondence principle apply to Wrappers in a Flex container?
 - Difference in Harness(Button) vs Harness(Flex(Button))
 - Flex alignment vs text alignment

- [ ] Refactor AppDelegate
 - Ideally, should be a single function (?)
 - Probably unify on_event, on_command, on_action

- [ ] Add WidgetBuilder trait
- [ ] Switch module file conventions (foobar/mod.rs -> _foobar.rs)
- [ ] Add dev shortcuts:
 - [ ] To print Widget tree
 - [ ] To print current focused/active Widget
 - [ ] To display invalidation rect

- [ ] Trait-ify Glazier
 - [ ] ??? Start at the bottom?

- [ ] Glazier
 - [ ] Rename WinHandler methods
  - size -> resize
  - scale -> rescale
  - command -> select_menu
  - save_as -> save_dialog_res
  - open_file -> open_dialog_res
  - timer  -> timer_complete

- [ ] Add replay mode
 - [ ] Record platform events
 - [ ] Record timers
 - [ ] Add way to rr tests

- [ ] Add ability to select rectangular region, get a list of Widget in return or something.

- [ ] Add "click all visible buttons" method to help test examples.

- [ ] Switch to ECS
- [ ] Remove WidgetId::reserved, move WidgetId::new
- [ ] Drag 'n Drop



- [ ] Add fuzzing
 - [ ] Check Masonry invariants
 - [ ] Create fuzzing suite
 - [ ] Test it with intentional bugs

- [ ] Add profiling
 - [ ] Add artinasal timings manager
 - [ ] Start in separate crate with artificial benchmarks
 - [ ] Add "median/worst FPS" log at the end of each run.

- [ ] Write benchmarks
 - [ ] Try to find ways to bust the framework's framerate
 - [ ] Huge list
 - [ ] Huge list with editable/focusable boxes
 - [ ] ???

- [ ] Refactor Command infrastructure
 - [ ] Broadcast (used multiple times)
 - [ ] Command (used once)
 - [ ] ???
 - [ ] Add on_command() trait method
 - [ ] Unify timers and promises

- [ ] Reintroduce Env type (?)
 - [ ] Remove Data trait
 - [ ] Make it so that required keys are known at launch time for better error messages.
 - [ ] Re-add Localization

- [ ] Get better coverage on Github Actions
 - Take inspiration from druid's Actions

- [ ] Improve how Harness mocks IME
 - [ ] Handle in-place editing (eg Kanji input, I think?).
 - [ ] Handle movement (eg ctrl+left).

- [ ] Merge WidgetMut and Ctx lifetimes
 - Note: probably requires better language support for implicit lifetime bounds

## Tests to write

- [X] event
- [X] promise
- [X] utils
- [X] widget_pod
 -> [X] 'active' status
 -> [X] debug_validate
 -> [X] check_initialized
 -> [X] check set_origin
 -> [X] MouseLeave
 -> [X] layout changes hot status
 -> [ ] paint_insets
 -> [ ] pan_to_child
 -> [ ] ParentWindowOrigin
 -> [ ] debug_widget_text
 -> [x] check for WidgetAdded
-> [X] widget utils
-> [X] Widgets
 -> [X] Button
 -> [X] Checkbox
 -> [X] Portal
 -> [X] Flex
 -> [X] Image
 -> [X] Label
 -> [X] SizedBox
 -> [X] Spinner
-> [ ] ObjectFit
-> [ ] text
 -> [ ] TextBox


# Design stuff

What referential are various values using?

transform_scroll translates mouse events

General note: add hover detection *everywhere*. It's what makes the UI feel alive.

## Flex stuff

Flex algo
- Does Tennent's Correspondence Principle apply to Wrappers in a Flex container?
- Visualize how much "spring" each element has

Difference between
- Window(Button)
- Window(Flex(Button))

- Flex(stuff)
- Portal(Flex(stuff))

Flex alignment vs text alignment

Portal.must_fill, Portal.constrain_horizontal, Portan.constrain_vertical

In druid Spinner needs parent constraints to decide its size.
Feels like that shouldn't be allowed.

Use "*It's a Wonderful World* boardgame"-inspired pass order.

See this thread: https://xi.zulipchat.com/#narrow/stream/147922-new-members/topic/Second.20book.20example.20doesn't.20work/

How does Flex interact with Viewport?
-> use call context for debugging (wait, what does that mean again?)

Make library of commonly desired layouts
- Center object vertically
- Horizontally
- Side gutters
- Document format

How do make easily-readable test of Flex, ObjectFit layout?


## Passes

Passes have access to
- on_event
-> mut self, mut ContextState, WidgetState.<layout>, mut WidgetState.<passes>, mut WidgetState.<Status>, get_widget_view()
- on_status_change
-> mut self, ContextState, WidgetState.<layout>, mut WidgetState.<passes>, WidgetState.<Status>
- lifecycle
-> mut self, mut ContextState, WidgetState.<layout>, mut WidgetState.<passes>, mut WidgetState.<Status>, get_widget_view()
- layout
-> self, mut ContextState, mut WidgetState.<layout>, WidgetState.<passes>, WidgetState.<Status>
- paint
-> self, mut ContextState, mut WidgetState.<layout>, WidgetState.<passes>, WidgetState.<Status>
- WidgetView
-> mut self, mut ContextState, WidgetState.<layout>, mut WidgetState.<passes>, mut WidgetState.<Status>, get_widget_view()

WidgetAdded and similar events are an anti-pattern. Constructors should stand on their own. (is it actually possible though?)


## Pointer status

Default(Option(cursor))
Active(Option(cursor))
Disabled
Hidden

See https://xi.zulipchat.com/#narrow/stream/147926-druid/topic/.22hot.22.20in.20the.20presence.20of.20multiple.20pointers/near/254524486


## Invariants

Druid invariants
- layout != <prev>.layout                       => request_layout() has been called
- child.origin != <prev>.child.origin           => request_layout() has been called
- <paint-calls> != <prev>.<paint-calls>         => request_paint() has been called
- children() != <prev>.children()               => children_changed is set
- !event.handled && hovered || self.has_focus   => recurse event
- lifecycle                                     => recurse lifecycle
- !is_hidden()                                  => recurse layout
- !is_hidden() && is_visible()                  => recurse paint
- is_focusable                                  => must be const
- ids are unique


## Profiling

Record spans, with:
- pass name
- widget id
- work type
- useful/redundant/testing
- include-recursed-time-in-total-time

-> percentage of work that is useful
-> what type of work


## Debug info

- Isolate subgraph of trace during errors

Event debug info:
- layout during event
- tree of visited widgets
- respective mouse pose

Widget Debugging
- Need to see the layout tree at any given point
- Need to see which subwidgets were not rendered and why
- Need to see the properties of each widget *at render time* both graphically and in text form

## Resource handling

See swift proposal https://github.com/apple/swift-evolution/blob/main/proposals/0271-package-manager-resources.md
