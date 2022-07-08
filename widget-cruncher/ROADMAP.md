# TODO - MVP

- [X] Clean up imports
- [X] Add impl Deref for WidgetRef
- [X] Add downcast to WidgetRef and WidgetView
- [X] Add WidgetAdded test
- [X] Rename ContextState to GlobalPassCtx
- [ ] Add WidgetView::as_ref
- [ ] Brainstorm names for AppDelegate, Harness methods, Action, ExtEvent

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

- [ ] Implement TextBox
 - [X] Implement Scroll
 - [X] Add text/IME to harness
 - [X] Port TextBox
 - [ ] Write unit tests

- [ ] Refactor WidgetState
 - [ ] Sort WidgetState fields
 - [ ] Add WidgetTmpState
 - [ ] Extract WidgetState fields to WidgetTmpState + global state
- [ ] Remove ExtendDrain

- [ ] Refactor ScrollComponent / paint_rect / paint_insets
 - [ ] No paint_insets, set paint_rect automatically or manually
 - [ ] Rework set_origin
 - [ ] Check that paint_rect has children paint_rect
 - [ ] Disambiguate concept of hidden/stashed widgets
 - [ ] Fix invalidation

- [ ] Fix WidgetView type
 - [ ] Add `View` associated type to Widget
 - [ ] Add impl DerefMut to WidgetView
 - [ ] Be able to create WidgetView from owned Widget

- [ ] Re-add Dialog
- [ ] Refactor TextLayout

- [ ] Write examples
 - [X] To-do List
 - [X] Fix blocking_function example
 - [ ] Rewrite Disney example
 - [ ] Copy examples from druid

- [ ] Rename to Masonry
- [ ] Rewrite License bits
- [ ] Write ARCHITECTURE.md
- [ ] Publish

# TODO - Long term

- [ ] Port Panoramix

- [ ] Move debug logger to use tracing infrastructure

- [ ] Refactor Mouse Events and hot status changes

- [ ] Remove `impl Widget for Box<dyn Widget>`

- [ ] Remove Data trait

- [ ] Refactor different passes
 - [ ] Write a WidgetCtx trait
 - [ ] Add a `WidgetPod::as_mut(ctx: &impl WidgetCtx)` method
 - [ ] Handle reques_layout-in-layout corner case

- [ ] Add dev shortcuts:
 - [ ] To print Widget tree
 - [ ] To print current focused/active Widget

- [ ] Write analysis of different layout footguns
 - Flex interacting with viewport
 - Does Tennent's Correspondence principle apply to Wrappers in a Flex container?
 - Difference in Harness(Button) vs Harness(Flex(Button))
 - Flex alignment vs text alignment

- [ ] Refactor AppDelegate
 - Ideally, should be a single function (?)
 - Probably unify on_event, on_command, on_action

- [ ] Trait-ify druid-shell
 - [ ] ??? Start at the bottom?

- [ ] Add replay mode
 - [ ] Record platform events
 - [ ] Record timers
 - [ ] Add way to rr tests

- [ ] Add fuzzing
 - [ ] Check Druid invariants
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

- [ ] Refactor Druid env
 - [ ] Remove Data trait
 - [ ] Make it so that required keys are known at launch time for better error messages.
 - [ ] Re-add Localization

- [ ] Improve how Harness mocks IME
 - [ ] Handle in-place editing (eg Kanji input, I think?).
 - [ ] Handle movement (eg ctrl+left).

- [ ] Switch to ECS
- [ ] Remove WidgetId::reserved, move WidgetId::new
- [ ] Drag 'n Drop

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
-> [ ] FillStrat
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

How does Flex interact with Viewport?
-> use call context for debugging (wait, what does that mean again?)

Make library of commonly desired layouts
- Center object vertically
- Horizontally
- Side gutters
- Document format

How do make easily-readable test of Flex, FillStrat layout?


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

Widget Debuggging
- Need to see the layout tree at any given point
- Need to see which subwidgets were not rendered and why
- Need to see the properties of each widget *at render time* both graphically and in text form

## Resource handling

See swift proposal https://github.com/apple/swift-evolution/blob/main/proposals/0271-package-manager-resources.md
