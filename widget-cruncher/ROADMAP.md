# Design stuff

What referential are various values using?

Flex algo
- Does Tennent's Correspondence Principle apply to Wrappers in a Flex container?
- Visualize how much "spring" each element has

transform_scroll translates mouse events

WidgetAdded and similar events are an anti-pattern. Constructors should stand on their own. (is it actually possible though?)



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




# Profiling

Record spans, with:
- pass name
- widget id
- work type
- useful/redundant/testing
- include-recursed-time-in-total-time

-> percentage of work that is useful
-> what type of work


# Trace handling

- Isolate subgraph of trace during errors



# TODO

- Write introspection features
 - Debug tree
 - Layout tree (serializable)
 - AppState
 - Logs

- Rewrite tests
 - Create coverage profile
 - Look at coverage of every file, write a list of target coverage
 - Add timers to harness

- Write examples
 - To do List
 - Rewrite Disney example

- Trait-ify druid-shell
 - ??? Start at the bottom?

- Add replay mode
 - Record platform events
 - Record timers
 - Add way to rr tests

- Add fuzzing
 - Check Druid invariants
 - Create fuzzing suite
 - Test it with intentional bugs

- Add profiling
 - Add artinasal timings manager
 - Start in separate crate with artificial benchmarks
 - Add "median/worst FPS" log at the end of each run.

- Write benchmarks
 - Try to find ways to bust the framework's framerate
 - Huge list
 - Huge list with editable/focusable boxes
 - ???

- Re-add Dialog
- Re-add ExtEvent + AppDelegate
- Refactor TextLayout
- Rewrite License bits
- Check ids are unique
- Getter downcast
- Rework set_origin
- Use pretty assert
- Rename ContextState to GlobalPassCtx
- Sort WidgetState fields
- Add WidgetTmpState
- Extract WidgetState fields to WidgetTmpState + global state
- Remove ExtendDrain
- Drag 'n Drop

- Refactor ScrollComponent
 - Disambiguate concept of hidden/stashed widgets
 - Fix invalidation

- Refactor Druid env
 - Remove Data trait
 - Make it so that required keys are known at launch time for better error messages.
 - Re-add Localization

- Refactor Command infrastructure
 - Broadcast (used multiple times)
 - Command (used once)
 - ???
 - Add on_command() trait method

- Use WidgetView everywhere
- Switch to ECS
- Port Panoramix
