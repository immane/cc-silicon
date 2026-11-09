// ============================================================================
// c53_layout_init_schema.rs — `/46` layout/init carrier acceptance.
//
// Pins the typed LayoutRecord/InitRecord wire shapes, symbolic extents,
// VLA/member descriptors, init steps/relocations, and append-body handles.
// No layout/init producer is claimed.
// ============================================================================

use cc_silicon_compiler::bus::{
    InitForm, InitRecord, InitStep, InitStorage, IntegerConst, LayoutExtent, LayoutForm, LayoutKey,
    LayoutMember, LayoutRecord, LayoutTarget, VlaDescriptor,
};
use cc_silicon_compiler::contract::{G1_DRAFT_BODY_NAMES, INSTRUCTION_RECORD_FIELDS};
use cc_silicon_compiler::ids::{ConstId, LayoutId, NameId, NodeId, ScopeId, SymbolId, TypeId};
use cc_silicon_compiler::snapshot::{decode_init, decode_layout, encode_init, encode_layout};

fn layout() -> LayoutRecord {
    LayoutRecord {
        key: LayoutKey {
            ty: TypeId::from_index(1),
            pack: Some(8),
            target: LayoutTarget::Probed([7u8; 32]),
        },
        form: LayoutForm::Array {
            element: LayoutId::from_index(2),
            count: Some(4),
        },
        size: LayoutExtent::Bits(128),
        align: LayoutExtent::Bits(32),
        members: vec![LayoutMember {
            index: 0,
            name: Some(NameId::from_index(3)),
            ty: TypeId::from_index(4),
            layout: LayoutId::from_index(5),
            offset: LayoutExtent::Bits(16),
            bit_width: Some(3),
            bit_offset: LayoutExtent::Bits(2),
        }],
        vla: Some(VlaDescriptor {
            bound: NodeId::from_index(6),
            scope: ScopeId::from_index(7),
            element: LayoutId::from_index(8),
            stride: LayoutExtent::Bits(64),
        }),
    }
}

fn init() -> InitRecord {
    InitRecord {
        ty: TypeId::from_index(1),
        layout: LayoutId::from_index(2),
        form: InitForm::Plan {
            storage: InitStorage::Automatic,
            steps: vec![
                InitStep::Zero {
                    offset: LayoutExtent::Bits(0),
                    length: LayoutExtent::Bits(8),
                },
                InitStep::Scalar {
                    offset: LayoutExtent::Bits(8),
                    value: ConstId::from_index(3),
                },
                InitStep::Relocation {
                    offset: LayoutExtent::Bits(16),
                    base: Some(SymbolId::from_index(4)),
                    addend: IntegerConst {
                        magnitude: vec![8],
                        negative: false,
                    },
                },
                InitStep::RuntimeStore {
                    offset: NodeId::from_index(5),
                    value: NodeId::from_index(6),
                },
            ],
        },
    }
}

#[test]
fn layout_and_init_round_trip() {
    let layout_record = layout();
    assert_eq!(
        decode_layout(&encode_layout(&layout_record)).unwrap(),
        layout_record
    );
    let init_record = init();
    assert_eq!(
        decode_init(&encode_init(&init_record)).unwrap(),
        init_record
    );
}

#[test]
fn append_body_inventory_includes_layout_and_init() {
    assert_eq!(G1_DRAFT_BODY_NAMES.len(), 20);
    assert_eq!(G1_DRAFT_BODY_NAMES[18], "layout");
    assert_eq!(G1_DRAFT_BODY_NAMES[19], "init");
    // The instruction field inventory remains the `/44` shape.
    assert!(INSTRUCTION_RECORD_FIELDS.contains(&"successors"));
}
