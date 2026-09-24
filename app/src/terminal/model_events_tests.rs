use warpui::App;

use super::*;
use crate::features::FeatureFlag;
use crate::terminal::model::session::IsSSHWrapperSession;

#[test]
fn ssh_wrapper_initializes_directly_without_remote_server() {
    assert_initializes_directly(IsSSHWrapperSession::Yes {
        socket_path: "/tmp/tilde-test-ssh.sock".into(),
        external_control_master: true,
    });
}

#[test]
fn local_shell_initializes_directly_without_remote_server() {
    assert_initializes_directly(IsSSHWrapperSession::No);
}

fn assert_initializes_directly(wrapper: IsSSHWrapperSession) {
    App::test((), |mut app| async move {
        let _flag = FeatureFlag::SshRemoteServer.override_enabled(true);
        let sessions = app.add_model(|_| Sessions::new_for_test());
        let dispatcher = app.add_model(|ctx| {
            ModelEventDispatcher::new(async_channel::unbounded().1, sessions, ctx)
        });
        let (tx, rx) = async_channel::unbounded();
        app.update(|ctx| {
            ctx.subscribe_to_model(&dispatcher, move |_, event, _| {
                let ModelEvent::Handler(AnsiHandlerEvent::InitShell {
                    pending_session_info,
                }) = event
                else {
                    panic!("expected ordinary InitShell");
                };
                tx.try_send(pending_session_info.clone()).unwrap();
            });
            dispatcher.update(ctx, |dispatcher, ctx| {
                let mut info = SessionInfo::new_for_test();
                info.session_id = 19.into();
                info.user = "local-user".into();
                info.is_ssh_wrapper_session = wrapper.clone();
                dispatcher.handle_terminal_model_event(
                    Event::Handler(HandlerEvent::InitShell {
                        pending_session_info: Box::new(info),
                    }),
                    ctx,
                );
            });
        });
        let pending_session_info = rx
            .try_recv()
            .expect("InitShell must reach the PTY without server interception");
        assert_eq!(pending_session_info.session_id, 19.into());
        assert_eq!(pending_session_info.user, "local-user");
        assert_eq!(pending_session_info.is_ssh_wrapper_session, wrapper);
        assert!(rx.try_recv().is_err());
    });
}
