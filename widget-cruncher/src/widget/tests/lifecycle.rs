use crate::testing::{Harness, Record, Recording, TestWidgetExt as _};
use crate::widget::{Label, SizedBox};
use crate::*;
use test_env_log::test;

#[test]
fn app_creation_lifecycle() {
    let record = Recording::default();
    let widget = SizedBox::empty().record(&record);

    let _harness = Harness::create(widget);

    assert!(matches!(record.next(), Record::L(LifeCycle::WidgetAdded)));
    assert!(matches!(
        record.next(),
        Record::L(LifeCycle::BuildFocusChain)
    ));
    assert!(matches!(record.next(), Record::E(Event::WindowConnected)));
    assert!(matches!(record.next(), Record::E(Event::WindowSize(_))));
    assert!(record.is_empty());
}
