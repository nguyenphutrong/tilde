use diesel::connection::SimpleConnection;
use diesel::{Connection, QueryDsl, RunQueryDsl, SqliteConnection};
use diesel_migrations::MigrationHarness;

use crate::{MIGRATIONS, schema};

#[test]
fn migrations_create_and_reopen_local_data_without_erasing_legacy_accounts() {
    let mut connection = SqliteConnection::establish(":memory:").unwrap();
    let applied = connection.run_pending_migrations(MIGRATIONS).unwrap();
    assert!(!applied.is_empty());

    connection
        .batch_execute(
            "INSERT INTO windows (id, active_tab_index) VALUES (19, 0);
             INSERT INTO users (id, firebase_uid) VALUES (7, 'legacy-offline-account');",
        )
        .unwrap();

    assert!(
        connection
            .run_pending_migrations(MIGRATIONS)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        schema::windows::table
            .select(schema::windows::id)
            .load::<i32>(&mut connection)
            .unwrap(),
        vec![19]
    );
    assert_eq!(
        schema::users::table
            .select(schema::users::firebase_uid)
            .load::<String>(&mut connection)
            .unwrap(),
        vec!["legacy-offline-account"]
    );
}
