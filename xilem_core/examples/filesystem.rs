// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! An example using Xilem Core to manipulate a filesystem.

use std::io::stdin;
use std::path::PathBuf;

use xilem_core::{
    AnyElement, AnyView, Mut, SuperElement, View, ViewElement, ViewId, ViewMarker, ViewPathTracker,
    environment::Environment,
};

#[derive(Debug)]
enum State {
    Setup,
    Empty,
    Complex(String),
}

fn complex_state(value: &str) -> impl FileView<State> + use<> {
    File {
        name: value.to_string(),
        contents: value.to_string(),
    }
}

fn app_logic(state: &mut State) -> impl FileView<State> + use<> {
    let res: DynFileView<State> = match state {
        State::Setup => Box::new(File {
            name: "file1.txt".into(),
            contents: "Test file contents".into(),
        }),
        State::Empty =>
        /* Box::new(Folder {
            name: "nothing".into(),
            seq: (),
        }) */
        {
            unimplemented!()
        }
        State::Complex(value) => Box::new(complex_state(value.as_str())),
    };
    res
}

fn main() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("examples/filesystem");
    if path.exists() {
        std::fs::remove_dir_all(&path).expect("Could create directory");
    }
    std::fs::create_dir(&path).expect("Could tidy up directory");
    let mut state = State::Setup;

    let mut previous = app_logic(&mut state);
    let mut input_buf = String::new();
    let mut root_ctx = ViewCtx {
        current_folder_path: path.clone(),
        view_path: Vec::new(),
        environment: Environment::new(),
    };
    let (mut element, mut initial_state) = previous.build(&mut root_ctx, &mut state);
    loop {
        input_buf.clear();
        let read_count = stdin()
            .read_line(&mut input_buf)
            .expect("Could read from stdin");
        if read_count == 0 {
            // Reached EOF, i.e. user has finished
            break;
        }
        input_buf.make_ascii_lowercase();
        let input = input_buf.trim();
        match input {
            "begin" => {
                state = State::Setup;
            }
            "clear" => {
                state = State::Empty;
            }
            complex if complex.starts_with("complex ") => {
                state = State::Complex(complex.strip_prefix("complex ").unwrap().into());
            }
            other => {
                eprint!("Unknown command {other:?}. Please try again:");
                continue;
            }
        };
        let new_view = app_logic(&mut state);
        root_ctx.current_folder_path.clone_from(&path);
        new_view.rebuild(
            &previous,
            &mut initial_state,
            &mut root_ctx,
            &mut element.0,
            &mut state,
        );
        previous = new_view;
    }
}

trait FileView<State, Action = ()>: View<State, Action, ViewCtx, Element = FsPath> {}

impl<V, State, Action> FileView<State, Action> for V where
    V: View<State, Action, ViewCtx, Element = FsPath>
{
}

type DynFileView<State, Action = ()> = Box<dyn AnyView<State, Action, ViewCtx, FsPath>>;

impl SuperElement<Self, ViewCtx> for FsPath {
    fn upcast(_ctx: &mut ViewCtx, child: Self) -> Self {
        child
    }

    fn with_downcast_val<R>(
        this: Self::Mut<'_>,
        f: impl FnOnce(Mut<'_, Self>) -> R,
    ) -> (Self::Mut<'_>, R) {
        let ret = f(this);
        (this, ret)
    }
}

impl AnyElement<Self, ViewCtx> for FsPath {
    fn replace_inner(this: Self::Mut<'_>, child: Self) -> Self::Mut<'_> {
        *this = child.0;
        this
    }
}

// Folder is meant to showcase ViewSequence, but isn't currently wired up
// struct Folder<Marker, Seq: ViewSequence<(), (), ViewCtx, FsPath, Marker>> {
//     name: String,
//     seq: Seq,
//     phantom: PhantomData<fn() -> Marker>,
// }

#[derive(Clone)]
struct File {
    name: String,
    contents: String,
}

struct FsPath(PathBuf);

impl From<PathBuf> for FsPath {
    fn from(value: PathBuf) -> Self {
        Self(value)
    }
}

impl ViewElement for FsPath {
    // TODO: This data is pretty redundant
    type Mut<'a> = &'a mut PathBuf;
}

impl ViewMarker for File {}
impl<State, Action> View<State, Action, ViewCtx> for File {
    type Element = FsPath;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let path = ctx.current_folder_path.join(&*self.name);

        // TODO: How to handle errors here?
        let _ = std::fs::write(&path, self.contents.as_bytes());
        (path.into(), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        _app_state: &mut State,
    ) {
        if prev.name != self.name {
            let new_path = ctx.current_folder_path.join(&*self.name);
            let _ = std::fs::rename(&*element, &new_path);
            *element = new_path;
        }
        if self.contents != prev.contents {
            let _ = std::fs::write(&*element, self.contents.as_bytes());
        }
    }

    fn teardown(
        &self,
        _view_state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        element: Mut<'_, Self::Element>,
        _app_state: &mut State,
    ) {
        let _ = std::fs::remove_file(element);
    }

    fn message(
        &self,
        _view_state: &mut Self::ViewState,
        _id_path: &[ViewId],
        _message: xilem_core::DynMessage,
        _app_state: &mut State,
    ) -> xilem_core::MessageResult<Action> {
        unreachable!()
    }
}

struct ViewCtx {
    view_path: Vec<ViewId>,
    current_folder_path: PathBuf,
    environment: Environment,
}

impl ViewPathTracker for ViewCtx {
    fn environment(&mut self) -> &mut Environment {
        &mut self.environment
    }
    fn push_id(&mut self, id: ViewId) {
        self.view_path.push(id);
    }
    fn pop_id(&mut self) {
        self.view_path.pop();
    }
    fn view_path(&mut self) -> &[ViewId] {
        &self.view_path
    }
}
