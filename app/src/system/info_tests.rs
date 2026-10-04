use std::cell::Cell;
use std::rc::Rc;

use byte_unit::Byte;

use super::*;
use crate::terminal::model::test_utils::TestBlockBuilder;
use crate::test_util::mock_blockgrid;

#[test]
fn excessive_memory_warning_requires_confirmation_and_emits_once_without_server() {
    App::test((), |mut app| async move {
        let model = app.add_model(SystemInfo::new);
        let warnings = Rc::new(Cell::new(0));
        app.update(|ctx| {
            let warnings = warnings.clone();
            ctx.subscribe_to_model(&model, move |_, event, _| {
                if matches!(event, SystemInfoEvent::MemoryUsageHigh) {
                    warnings.set(warnings.get() + 1);
                }
            });
        });

        let threshold = 10_000_000_000;
        for (footprint, expected_warnings) in [
            (threshold, 0),
            (threshold - 1, 0),
            (threshold + 1, 0),
            (threshold, 1),
            (threshold * 2, 1),
        ] {
            model.update(&mut app, |model, ctx| {
                model.check_for_excessive_memory_usage(Byte::from_u64(footprint), ctx);
            });
            assert_eq!(warnings.get(), expected_warnings);
        }
    });
}

#[test]
fn test_memory_usage_stats_construction() {
    let total_application_usage_bytes = 1024;
    let mut stats = MemoryUsageStats::new(Byte::from_u64(total_application_usage_bytes));

    let now = Local::now();

    let mut block_with_content = TestBlockBuilder::new().build();
    block_with_content.set_prompt_and_command_grid(mock_blockgrid("line1\nline2"));
    block_with_content.set_output_grid(mock_blockgrid("line3"));
    block_with_content.update_last_painted_at(now);

    let inactive_5m_block = TestBlockBuilder::new().build();
    inactive_5m_block.update_last_painted_at(now - chrono::Duration::minutes(10));

    let inactive_1h_block1 = TestBlockBuilder::new().build();
    inactive_1h_block1.update_last_painted_at(now - chrono::Duration::minutes(70));

    let inactive_1h_block2 = TestBlockBuilder::new().build();
    inactive_1h_block2.update_last_painted_at(now - chrono::Duration::minutes(70));

    let blocks = [
        block_with_content,
        inactive_5m_block,
        inactive_1h_block1,
        inactive_1h_block2,
        TestBlockBuilder::new().build(),
    ];

    stats.add_blocks(now, blocks.iter());

    assert_eq!(
        stats.total_application_usage_bytes,
        total_application_usage_bytes as usize
    );
    assert_eq!(stats.total_blocks, 5);
    assert_eq!(stats.total_lines, 3);

    assert_eq!(stats.active_block_stats.num_blocks, 1);
    assert_eq!(stats.active_block_stats.num_lines, 3);

    assert_eq!(stats.inactive_5m_stats.num_blocks, 1);
    assert_eq!(stats.inactive_5m_stats.num_lines, 0);

    assert_eq!(stats.inactive_1h_stats.num_blocks, 2);
    assert_eq!(stats.inactive_1h_stats.num_lines, 0);

    assert_eq!(stats.inactive_24h_stats.num_blocks, 1);
    assert_eq!(stats.inactive_24h_stats.num_lines, 0);
}
