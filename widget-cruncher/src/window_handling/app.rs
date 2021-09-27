// Copyright 2019 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Window building and app lifecycle.

use crate::ext_event::{ExtEventHost, ExtEventSink};
use crate::kurbo::{Point, Size};
use crate::widget::LabelText;
use crate::win_handler::{AppHandler, AppState};
use crate::window::WindowId;
use crate::{Data, Env, LocalizedString, Widget};

use druid_shell::{Application, Error as PlatformError, WindowBuilder, WindowHandle, WindowLevel};
use druid_shell::WindowState;
