// Port of .upstream/zigbee-herdsman/test/adapter/z-stack/unpi.test.ts

use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::unpi::frame::Frame;
use zigbee_apiarist::adapter::z_stack::unpi::parser::Parser;
use zigbee_apiarist::adapter::z_stack::unpi::writer::Writer;

#[test]
#[ignore = "not implemented yet"]
fn parse_simple_message() {
    let buffer = vec![
        0xfe, 0x0e, 0x61, 0x02, 0x02, 0x00, 0x02, 0x06, 0x03, 0xd9, 0x14, 0x34, 0x01, 0x02, 0x00,
        0x00, 0x00, 0x00, 0x92,
    ];
    let mut parser = Parser::new();
    let parsed = parser.transform(&buffer);

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].r#type, Type::SRSP);
    assert_eq!(parsed[0].subsystem, Subsystem::SYS);
    assert_eq!(parsed[0].commandID, 2);
    assert_eq!(
        parsed[0].data,
        vec![2, 0, 2, 6, 3, 217, 20, 52, 1, 2, 0, 0, 0, 0]
    );
    assert_eq!(parsed[0].length, 14);
    assert_eq!(parsed[0].fcs, 0x92);
}

#[test]
#[ignore = "not implemented yet"]
fn parse_two_messages() {
    let buffer = vec![
        0xfe, 0x0e, 0x61, 0x02, 0x02, 0x00, 0x02, 0x06, 0x03, 0xd9, 0x14, 0x34, 0x01, 0x02, 0x00,
        0x00, 0x00, 0x00, 0x92, 0xfe, 0x03, 0x61, 0x08, 0x00, 0x01, 0x55, 0x3e,
    ];
    let mut parser = Parser::new();
    let parsed = parser.transform(&buffer);

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].r#type, Type::SRSP);
    assert_eq!(parsed[0].subsystem, Subsystem::SYS);
    assert_eq!(parsed[0].commandID, 2);
    assert_eq!(
        parsed[0].data,
        vec![2, 0, 2, 6, 3, 217, 20, 52, 1, 2, 0, 0, 0, 0]
    );
    assert_eq!(parsed[0].length, 14);
    assert_eq!(parsed[0].fcs, 0x92);
    assert_eq!(parsed[1].r#type, Type::SRSP);
    assert_eq!(parsed[1].subsystem, Subsystem::SYS);
    assert_eq!(parsed[1].commandID, 8);
    assert_eq!(parsed[1].data, vec![0, 1, 85]);
    assert_eq!(parsed[1].length, 3);
    assert_eq!(parsed[1].fcs, 0x3e);
}

#[test]
#[ignore = "not implemented yet"]
fn dont_throw_error_on_fcs_mismatch() {
    let buffer = vec![0xfe, 0x03, 0x61, 0x08, 0x00, 0x01, 0x55, 0x3f];
    let mut parser = Parser::new();
    assert!(parser.transform(&buffer).is_empty());
}

#[test]
#[ignore = "not implemented yet"]
fn message_in_two_chunks() {
    let mut parser = Parser::new();
    assert!(parser
        .transform(&[0xfe, 0x03, 0x61, 0x08, 0x00, 0x01])
        .is_empty());
    let parsed = parser.transform(&[0x55, 0x3e]);

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].r#type, Type::SRSP);
    assert_eq!(parsed[0].subsystem, Subsystem::SYS);
    assert_eq!(parsed[0].commandID, 8);
    assert_eq!(parsed[0].data, vec![0, 1, 85]);
    assert_eq!(parsed[0].length, 3);
    assert_eq!(parsed[0].fcs, 0x3e);
}

#[test]
#[ignore = "not implemented yet"]
fn message_in_two_chunks_fcs_as_separate() {
    let mut parser = Parser::new();
    assert!(parser
        .transform(&[0xfe, 0x03, 0x61, 0x08, 0x00, 0x01, 0x55])
        .is_empty());
    let parsed = parser.transform(&[0x3e]);

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].commandID, 8);
    assert_eq!(parsed[0].data, vec![0, 1, 85]);
}

#[test]
#[ignore = "not implemented yet"]
fn parse_message_when_it_does_not_start_with_sof() {
    let buffer = vec![
        95, 27, 37, 254, 3, 69, 196, 212, 23, 0, 65, 254, 27, 68, 129, 0, 0, 8, 0, 212, 23, 1, 1,
        0, 55, 0, 153, 178, 219, 0, 0, 7, 8, 122, 10, 0, 0, 32, 243, 212, 23, 29, 160, 254, 7, 69,
        196, 111, 244, 2, 122, 155, 246, 95, 87, 254, 27, 68, 129, 0, 0, 6, 0, 111, 244, 1, 1, 0,
        118, 0, 245, 236, 220, 0, 0, 7, 8, 2, 10, 0, 0, 16, 0, 246, 95, 27, 85,
    ];
    let mut parser = Parser::new();
    let parsed = parser.transform(&buffer);

    assert_eq!(parsed.len(), 4);
    assert_eq!(parsed[0].r#type, Type::AREQ);
    assert_eq!(parsed[0].subsystem, Subsystem::ZDO);
    assert_eq!(parsed[0].commandID, 196);
    assert_eq!(parsed[0].data, vec![0xd4, 0x17, 0x00]);
    assert_eq!(parsed[0].length, 3);
    assert_eq!(parsed[0].fcs, 65);
    assert_eq!(parsed[1].r#type, Type::AREQ);
    assert_eq!(parsed[1].subsystem, Subsystem::AF);
    assert_eq!(parsed[1].commandID, 129);
    assert_eq!(parsed[1].length, 27);
    assert_eq!(parsed[1].fcs, 160);
}

#[test]
#[ignore = "not implemented yet"]
fn continue_parsing_on_fcs_mismatch() {
    let mut parser = Parser::new();
    parser.transform(&[
        0x01, 0x02, 0xfe, 0x03, 0x61, 0x08, 0x00, 0x01, 0x55, 0x3f, 0x08, 0x09, 0x12,
    ]);
    parser.transform(&[0x08, 0x09, 0x12]);
    parser.transform(&[0x08, 0x09, 0x12, 0xfe, 0x0e, 0x61, 0x02]);
    let parsed = parser.transform(&[
        0x02, 0x00, 0x02, 0x06, 0x03, 0xd9, 0x14, 0x34, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x92,
    ]);

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].r#type, Type::SRSP);
    assert_eq!(parsed[0].subsystem, Subsystem::SYS);
    assert_eq!(parsed[0].commandID, 2);
    assert_eq!(
        parsed[0].data,
        vec![2, 0, 2, 6, 3, 217, 20, 52, 1, 2, 0, 0, 0, 0]
    );
    assert_eq!(parsed[0].length, 14);
    assert_eq!(parsed[0].fcs, 0x92);
}

#[test]
#[ignore = "not implemented yet"]
fn frame_to_buffer() {
    let frame = Frame::new(Type::SRSP, Subsystem::SYS, 3, vec![0x06, 0x01], 0, 0);
    assert_eq!(
        frame.to_buffer(),
        vec![0xfe, 0x02, 0x61, 0x03, 0x06, 0x01, 0x67]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn write_frame() {
    let frame = Frame::new(Type::SRSP, Subsystem::SYS, 3, vec![0x06, 0x01], 0, 0);
    let mut writer = Writer::default();
    writer.write_frame(&frame);
    assert_eq!(writer.pushed.len(), 1);
    assert_eq!(writer.pushed[0], frame.to_buffer());
}

#[test]
#[ignore = "not implemented yet"]
fn write_buffer() {
    let buffer = vec![0x01, 0x02];
    let mut writer = Writer::default();
    writer.write_buffer(&buffer);
    assert_eq!(writer.pushed.len(), 1);
    assert_eq!(writer.pushed[0], buffer);
}

#[test]
#[ignore = "not implemented yet"]
fn read_should_do_nothing() {
    let mut writer = Writer::default();
    writer.read();
}
