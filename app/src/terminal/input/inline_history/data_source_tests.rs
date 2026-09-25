use std::collections::HashSet;

use chrono::{Duration, Local};
use warpui::App;

use super::*;
use crate::terminal::history::{HistoryEntry, HistoryEvent};
use crate::terminal::model::session::{SessionId, SessionInfo, Sessions};
use crate::terminal::model_events::ModelEventDispatcher;

#[test]
fn shell_history_orders_dedupes_and_filters_without_ai_models() {
    App::test((), |mut app| async move {
        let other_session = SessionId::from(29);
        let session_id = SessionId::from(17);
        let sessions = app.add_model(|_| Sessions::new_for_test());
        let history = app.add_singleton_model(|_| History::default());
        let now = Local::now();
        let entry = |id, command: &str, offset| {
            HistoryEntry::command_at_time(
                command.to_owned(),
                now + Duration::seconds(offset),
                Some(id),
                false,
            )
        };
        for id in [other_session, session_id] {
            sessions.update(&mut app, |sessions, _| {
                let mut info = SessionInfo::new_for_test();
                info.session_id = id;
                sessions.register_session_for_test(info);
            });
            let session = sessions.read(&app, |sessions, _| sessions.get(id).unwrap());
            let (initialized_tx, initialized_rx) = async_channel::bounded(1);
            app.update(|ctx| {
                ctx.subscribe_to_model(&history, move |_, event, _| {
                    if matches!(event, HistoryEvent::Initialized(initialized) if *initialized == id)
                    {
                        let _ = initialized_tx.try_send(());
                    }
                });
                history.update(ctx, |history, ctx| {
                    history.init_session_with(session, async { Vec::new() }, ctx);
                });
            });
            initialized_rx.recv().await.unwrap();
            if id == other_session {
                history.update(&mut app, |history, _| {
                    history.append_commands(id, vec![entry(id, "echo external", -10)]);
                });
            }
        }
        history.update(&mut app, |history, _| {
            history.append_commands(
                session_id,
                vec![
                    entry(session_id, " echo newest ", 0),
                    entry(session_id, "printf oldest", 1),
                    entry(session_id, "echo 日本語", 2),
                    entry(session_id, "   ", 3),
                    entry(session_id, "echo newest", 4),
                ],
            );
            history.append_commands(other_session, vec![entry(other_session, "echo hidden", 5)]);
        });
        let (_tx, rx) = async_channel::unbounded();
        let dispatcher = app.add_model(|ctx| {
            let mut dispatcher = ModelEventDispatcher::new(rx, sessions.clone(), ctx);
            dispatcher.set_active_session_id(session_id);
            dispatcher
        });
        let active_session = app.add_model(|ctx| ActiveSession::new(sessions, dispatcher, ctx));
        let source = InlineHistoryMenuDataSource::new(EntityId::new(), active_session);
        for (prefix, expected) in [
            (
                "",
                vec![
                    "echo external",
                    "printf oldest",
                    "echo 日本語",
                    "echo newest",
                ],
            ),
            (
                "  echo  ",
                vec!["echo external", "echo 日本語", "echo newest"],
            ),
            ("echo 日", vec!["echo 日本語"]),
            ("Echo", vec![]),
        ] {
            let results = app.read(|ctx| {
                source
                    .run_query(
                        &Query {
                            text: prefix.to_owned(),
                            filters: HashSet::from([QueryFilter::Commands]),
                        },
                        ctx,
                    )
                    .unwrap()
            });
            assert_eq!(
                results
                    .iter()
                    .map(|item| item.accept_result().command)
                    .collect::<Vec<_>>(),
                expected
            );
            for (index, result) in results.iter().enumerate() {
                assert_eq!(result.score(), OrderedFloat(index as f64));
                assert_eq!(
                    result.accessibility_label(),
                    format!("Command: {}", expected[index])
                );
            }
        }
        app.read(|ctx| {
            assert!(
                source
                    .run_query(
                        &Query {
                            text: String::new(),
                            filters: HashSet::from([QueryFilter::PromptHistory]),
                        },
                        ctx
                    )
                    .unwrap()
                    .is_empty()
            )
        });
    });
}
