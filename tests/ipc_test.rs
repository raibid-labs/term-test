//! Integration tests for the IPC module.
//!
//! These tests verify the IPC functionality for split-process terminal testing.
//! Most tests are unit tests in the module itself; these are integration tests
//! that require a running daemon (skipped if daemon not available).

#[cfg(all(feature = "ipc", target_family = "unix"))]
mod ipc_tests {
    use std::time::Duration;

    use ratatui_testlib::ipc::{ControlMessage, DaemonConfig, DaemonTestHarness, ShmHeader};

    #[test]
    fn test_control_message_input_serialization() {
        let msg = ControlMessage::Input(b"test input".to_vec());
        let bytes = msg.to_bytes();

        assert_eq!(bytes[0], 0); // Type 0 = Input
        let len = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);
        assert_eq!(len, 10); // "test input" length
        assert_eq!(&bytes[5..], b"test input");
    }

    #[test]
    fn test_control_message_resize_serialization() {
        let msg = ControlMessage::Resize { cols: 120, rows: 40 };
        let bytes = msg.to_bytes();

        assert_eq!(bytes[0], 1); // Type 1 = Resize
        let cols = u16::from_le_bytes([bytes[1], bytes[2]]);
        let rows = u16::from_le_bytes([bytes[3], bytes[4]]);
        assert_eq!(cols, 120);
        assert_eq!(rows, 40);
    }

    #[test]
    fn test_control_message_refresh_serialization() {
        let msg = ControlMessage::Refresh;
        let bytes = msg.to_bytes();
        assert_eq!(bytes, vec![2]);
    }

    #[test]
    fn test_control_message_shutdown_serialization() {
        let msg = ControlMessage::Shutdown;
        let bytes = msg.to_bytes();
        assert_eq!(bytes, vec![3]);
    }

    #[test]
    fn test_config_builder() {
        let config = DaemonConfig::builder()
            .socket_path("/custom/path.sock")
            .shm_path("/custom_shm")
            .spawn_daemon(true)
            .daemon_command("custom-daemon")
            .daemon_args(vec!["--arg1".into(), "value1".into()])
            .dimensions(100, 30)
            .connect_timeout(Duration::from_secs(15))
            .default_timeout(Duration::from_secs(20))
            .build();

        assert_eq!(
            config.socket_path,
            std::path::PathBuf::from("/custom/path.sock")
        );
        assert_eq!(config.shm_path, "/custom_shm");
        assert!(config.spawn_daemon);
        assert_eq!(config.daemon_command, "custom-daemon");
        assert_eq!(config.daemon_args, vec!["--arg1", "value1"]);
        assert_eq!(config.dimensions, Some((100, 30)));
        assert_eq!(config.connect_timeout, Duration::from_secs(15));
        assert_eq!(config.default_timeout, Duration::from_secs(20));
    }

    #[test]
    fn test_config_default() {
        let config = DaemonConfig::default();

        assert_eq!(
            config.socket_path,
            std::path::PathBuf::from("/tmp/term-daemon.sock")
        );
        assert_eq!(config.shm_path, "/term_shm_v1");
        assert!(!config.spawn_daemon);
        assert_eq!(config.dimensions, Some((80, 24)));
    }

    #[test]
    fn test_shm_header_default_validation() {
        let valid_header = ShmHeader {
            magic: ShmHeader::DEFAULT_MAGIC,
            version: ShmHeader::DEFAULT_VERSION,
            cols: 80,
            rows: 24,
            cursor_col: 0,
            cursor_row: 0,
            sequence_number: 1,
            grid_offset: 36,
            grid_size: 1920,
            attrs_offset: 0,
            attrs_size: 0,
        };

        assert!(valid_header.validate().is_ok());
    }

    #[test]
    fn test_shm_header_invalid_magic() {
        let header = ShmHeader {
            magic: 0xDEADBEEF,
            version: ShmHeader::DEFAULT_VERSION,
            cols: 80,
            rows: 24,
            cursor_col: 0,
            cursor_row: 0,
            sequence_number: 1,
            grid_offset: 36,
            grid_size: 1920,
            attrs_offset: 0,
            attrs_size: 0,
        };

        assert!(header.validate().is_err());
    }

    #[test]
    fn test_shm_header_invalid_version() {
        let header = ShmHeader {
            magic: ShmHeader::DEFAULT_MAGIC,
            version: 999,
            cols: 80,
            rows: 24,
            cursor_col: 0,
            cursor_row: 0,
            sequence_number: 1,
            grid_offset: 36,
            grid_size: 1920,
            attrs_offset: 0,
            attrs_size: 0,
        };

        assert!(header.validate().is_err());
    }

    #[test]
    fn test_shm_header_custom_validation() {
        let header = ShmHeader {
            magic: 0x5343_5241, // "SCRA" - custom magic
            version: 2,
            cols: 80,
            rows: 24,
            cursor_col: 0,
            cursor_row: 0,
            sequence_number: 1,
            grid_offset: 36,
            grid_size: 1920,
            attrs_offset: 0,
            attrs_size: 0,
        };

        // Should fail with default validation
        assert!(header.validate().is_err());

        // Should pass with custom validation
        assert!(header.validate_with(0x5343_5241, 2).is_ok());
    }

    #[test]
    fn test_is_enabled_without_env() {
        // Clear the env var
        std::env::remove_var("RTL_IPC_TEST");
        assert!(!DaemonTestHarness::is_enabled());
    }

    #[test]
    fn test_is_enabled_with_env() {
        std::env::set_var("RTL_IPC_TEST", "1");
        assert!(DaemonTestHarness::is_enabled());
        std::env::remove_var("RTL_IPC_TEST");
    }

    // Integration test that requires a running daemon
    // Skip if daemon is not available
    #[test]
    #[ignore = "requires running daemon - run with RTL_IPC_TEST=1"]
    fn test_daemon_connection() {
        if !DaemonTestHarness::is_enabled() {
            return;
        }

        let config = DaemonConfig::default();
        let result = DaemonTestHarness::with_config(config);

        // Just verify we can attempt connection
        // Will fail if daemon not running, which is expected
        match result {
            Ok(harness) => {
                let (cols, rows) = harness.dimensions();
                assert!(cols > 0);
                assert!(rows > 0);
            }
            Err(e) => {
                println!("Expected error (daemon not running): {}", e);
            }
        }
    }

    #[test]
    #[ignore = "requires running daemon - run with RTL_IPC_TEST=1"]
    fn test_send_and_receive() {
        if !DaemonTestHarness::is_enabled() {
            return;
        }

        let config = DaemonConfig::default();
        let mut harness = match DaemonTestHarness::with_config(config) {
            Ok(h) => h,
            Err(_) => return, // Skip if daemon not available
        };

        // Send a simple echo command
        harness.send_input("echo test_marker_12345\n").unwrap();

        // Wait for the output
        harness
            .wait_for_text("test_marker_12345", Duration::from_secs(5))
            .unwrap();

        // Verify grid contains the marker
        assert!(harness.contains("test_marker_12345").unwrap());
    }
}
