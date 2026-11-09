// ============================================================================
// c52_type_schema.rs — `/46` T06 type-model append acceptance.
//
// Pins pointer/array/aggregate/enum type carriers, qualifiers, VLA bounds,
// aggregate members, and enum references through the canonical type codec.
// No T06 producer is claimed.
// ============================================================================

use cc_silicon_compiler::bus::{
    AggregateKind, AggregateMember, ArrayBound, Enumerator, TypeKind, TypeQualifiers, TypeRecord,
};
use cc_silicon_compiler::contract::{
    AGGREGATE_KIND_NAMES, AGGREGATE_MEMBER_FIELDS, ARRAY_BOUND_NAMES, ENUMERATOR_FIELDS,
    TYPE_KIND_NAMES, TYPE_QUALIFIER_FIELDS,
};
use cc_silicon_compiler::ids::{ConstId, NameId, NodeId, ScopeId, TypeId};
use cc_silicon_compiler::snapshot::{decode_type, encode_type};

#[test]
fn type_inventory_and_payload_fields_are_appended() {
    assert_eq!(
        TYPE_KIND_NAMES,
        &[
            "void",
            "bool",
            "char",
            "int",
            "function",
            "pointer",
            "array",
            "aggregate",
            "enum",
        ]
    );
    assert_eq!(
        TYPE_QUALIFIER_FIELDS,
        &["const", "volatile", "restrict", "atomic"]
    );
    assert_eq!(ARRAY_BOUND_NAMES, &["constant", "incomplete", "vla"]);
    assert_eq!(AGGREGATE_KIND_NAMES, &["struct", "union"]);
    assert_eq!(
        AGGREGATE_MEMBER_FIELDS,
        &["name", "ty", "bit_width", "flexible"]
    );
    assert_eq!(ENUMERATOR_FIELDS, &["name", "value"]);
}

#[test]
fn appended_type_kinds_round_trip() {
    let pointer = TypeRecord {
        kind: TypeKind::Pointer {
            pointee: TypeId::from_index(1),
            qualifiers: TypeQualifiers {
                is_const: true,
                is_volatile: false,
                is_restrict: true,
                is_atomic: false,
            },
        },
    };
    assert_eq!(decode_type(&encode_type(&pointer)).unwrap(), pointer);

    let array = TypeRecord {
        kind: TypeKind::Array {
            element: TypeId::from_index(2),
            bound: ArrayBound::Vla {
                bound: NodeId::from_index(3),
                scope: ScopeId::from_index(4),
            },
        },
    };
    assert_eq!(decode_type(&encode_type(&array)).unwrap(), array);

    let aggregate = TypeRecord {
        kind: TypeKind::Aggregate {
            kind: AggregateKind::Struct,
            members: vec![
                AggregateMember {
                    name: Some(NameId::from_index(5)),
                    ty: TypeId::from_index(6),
                    bit_width: None,
                    flexible: false,
                },
                AggregateMember {
                    name: None,
                    ty: TypeId::from_index(7),
                    bit_width: Some(3),
                    flexible: false,
                },
            ],
        },
    };
    assert_eq!(decode_type(&encode_type(&aggregate)).unwrap(), aggregate);

    let enumeration = TypeRecord {
        kind: TypeKind::Enum {
            underlying: TypeId::from_index(8),
            enumerators: vec![Enumerator {
                name: NameId::from_index(9),
                value: Some(ConstId::from_index(10)),
            }],
        },
    };
    assert_eq!(
        decode_type(&encode_type(&enumeration)).unwrap(),
        enumeration
    );
}

#[test]
fn incomplete_and_constant_array_bounds_round_trip() {
    for bound in [
        ArrayBound::Incomplete,
        ArrayBound::Constant(12),
        ArrayBound::Vla {
            bound: NodeId::from_index(0),
            scope: ScopeId::from_index(0),
        },
    ] {
        let record = TypeRecord {
            kind: TypeKind::Array {
                element: TypeId::from_index(0),
                bound,
            },
        };
        assert_eq!(decode_type(&encode_type(&record)).unwrap(), record);
    }
}
