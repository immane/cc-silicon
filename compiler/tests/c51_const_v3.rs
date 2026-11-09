// ============================================================================
// c51_const_v3.rs — `/45` constant v3 append acceptance.
//
// Pins the append-only float/address tags, their canonical snapshot payloads,
// and the tag/fields inventories. No float or address producer is claimed.
// ============================================================================

use cc_silicon_compiler::bus::{AddressConst, ConstRecord, ConstValue, FloatConst, IntegerConst};
use cc_silicon_compiler::codec::CodecError;
use cc_silicon_compiler::contract::{ADDRESS_CONST_FIELDS, CONST_VALUE_NAMES, FLOAT_CONST_FIELDS};
use cc_silicon_compiler::ids::{SymbolId, TypeId};
use cc_silicon_compiler::snapshot::{decode_const, encode_const};
use cc_silicon_compiler::target::FloatFormat;

#[test]
fn const_value_inventory_appends_float_and_address() {
    assert_eq!(CONST_VALUE_NAMES, &["integer", "float", "address"]);
    assert_eq!(FLOAT_CONST_FIELDS, &["format", "bits"]);
    assert_eq!(ADDRESS_CONST_FIELDS, &["base", "addend"]);
}

#[test]
fn float_and_address_payloads_round_trip() {
    let float = ConstRecord {
        ty: TypeId::from_index(2),
        value: ConstValue::Float(FloatConst {
            format: FloatFormat::IeeeBinary64,
            bits: vec![0x3f, 0xf0, 0, 0, 0, 0, 0, 0],
        }),
        origins: vec![],
    };
    assert_eq!(decode_const(&encode_const(&float)).unwrap(), float);

    let address = ConstRecord {
        ty: TypeId::from_index(3),
        value: ConstValue::Address(AddressConst {
            base: Some(SymbolId::from_index(4)),
            addend: IntegerConst {
                magnitude: vec![8],
                negative: false,
            },
        }),
        origins: vec![],
    };
    assert_eq!(decode_const(&encode_const(&address)).unwrap(), address);
}

#[test]
fn noncanonical_float_bits_are_rejected() {
    let bad = ConstRecord {
        ty: TypeId::from_index(2),
        value: ConstValue::Float(FloatConst {
            format: FloatFormat::IeeeBinary64,
            bits: vec![0, 0],
        }),
        origins: vec![],
    };
    assert!(matches!(
        decode_const(&encode_const(&bad)),
        Err(CodecError::Unsupported(_))
    ));
}
