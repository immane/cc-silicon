// ============================================================================
// chips/lx_decode.rs — T04 LX05–LX08 literal-decode worker
// (Wave 2 slice 2, `/11`)
//
// Reads one committed C token plus its committed PP token
// (payload: exactly `[Token, PpToken]`), appends one `LiteralRecord` with
// `token: Some(committed TokenId)`, and completes with its predicted
// reference. M1 exercised scope only: decimal digits, no suffix, `Int`
// candidate, unsigned. Anything else fails with explicit `Unsupported`.
// The decode input is the committed PP spelling/kind (DOC-10 resolution);
// the C-token relation is the output-side publish-time back-link.
// ============================================================================

use crate::chips::{fail, protocol_fault, Worker};
use crate::bus::{PpTokenKind, TokenKind};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, RecordRef, TaskId, TokenId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, LX_DECODE_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the literal-decode computation.
#[derive(Clone, Debug)]
pub struct LxDecodeInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Committed C token under decode.
    pub token: TokenId,
    /// Its committed kind (must be `Integer`).
    pub token_kind: TokenKind,
    /// Committed PP spelling bytes.
    pub spelling: Vec<u8>,
    /// `literals` arena count at dispatch (single-append prediction base).
    pub literals_allocated: u32,
}

/// Build the narrow projection for one dispatched task.
pub fn project_lx_decode_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<LxDecodeInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("decode of unknown task {}", task.index())))?;
    if record.kind != TaskKind::LEX_DECODE_LITERAL {
        return Err(protocol_fault(format!(
            "decode task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 2 {
        return Err(protocol_fault(format!(
            "decode task {} payload must be exactly [Token, PpToken], got {} refs",
            task.index(),
            record.payload.refs.len()
        )));
    }
    let (Some(token), Some(pp_token)) = (
        match record.payload.refs[0] {
            RecordRef::Token(id) => Some(id),
            _ => None,
        },
        match record.payload.refs[1] {
            RecordRef::PpToken(id) => Some(id),
            _ => None,
        },
    ) else {
        return Err(protocol_fault(format!(
            "decode task {} payload must be exactly [Token, PpToken]",
            task.index()
        )));
    };
    let token_body = bus.arenas.tokens.get(token).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("decode reads missing token {}", token.index()),
        )
    })?;
    let pp_body = bus.arenas.pp_tokens.get(pp_token).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("decode reads missing pp-token {}", pp_token.index()),
        )
    })?;
    if pp_body.kind != PpTokenKind::PpNumber {
        return Err(DiagnosticDraft::unsupported(
            "literal decode reads a non-pp-number pp-token",
        ));
    }
    Ok(LxDecodeInput {
        task,
        state: record.state.clone(),
        token,
        token_kind: token_body.kind,
        spelling: pp_body.spelling.clone(),
        literals_allocated: bus.arenas.literals.allocated(),
    })
}

/// The T04 literal-decode worker (LX05–LX08 integer slice scope).
pub struct LxDecodeLiteralChip;

impl Worker for LxDecodeLiteralChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: LX_DECODE_CHIP,
            chip_name: "LxDecodeLiteralChip",
            group: TaskGroup::LEX,
            task_kinds: vec![TaskKind::LEX_DECODE_LITERAL],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Lex, "literals"),
            ],
            writes: vec![FieldPath::new(StoreId::Lex, "literals")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c11_lex.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_lx_decode_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl LxDecodeLiteralChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &LxDecodeInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("decode task {} is not running", input.task.index()),
                ),
            )];
        }
        if input.token_kind != TokenKind::Integer {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("decode of a non-integer token"),
            )];
        }
        if input.spelling.is_empty() || !input.spelling.iter().all(|byte| byte.is_ascii_digit()) {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported(
                    "literal outside the M1 exercised subset (decimal digits only)",
                ),
            )];
        }
        let value = decimal_magnitude(&input.spelling);
        let predicted = LiteralId::from_index(input.literals_allocated);
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![RecordDraft {
                        family: crate::ids::RecordFamily::Literal,
                        index: DraftRef(0),
                    }],
                    bodies: vec![G1DraftBody::Literal(crate::bus::LiteralRecord {
                        token: Some(input.token),
                        kind: LiteralKind::Integer,
                        radix: 10,
                        suffix: LiteralSuffix::None,
                        value,
                        negative: false,
                        spelling: input.spelling.clone(),
                        candidate_type: Lx08CandidateType::Int,
                    })],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Literal(predicted)),
            },
        ]
    }
}

/// Decimal digit string to minimal big-endian magnitude bytes (at least one
/// byte). Mechanical base-10 accumulation with no host-width overflow: an
/// `n`-digit input always fits, digit by digit.
pub fn decimal_magnitude(digits: &[u8]) -> Vec<u8> {
    let mut little: Vec<u8> = vec![0];
    for digit in digits {
        let mut carry = (digit - b'0') as u16;
        for byte in little.iter_mut() {
            let value = (*byte as u16) * 10 + carry;
            *byte = (value & 0xff) as u8;
            carry = value >> 8;
        }
        while carry > 0 {
            little.push((carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    while little.len() > 1 && little.last() == Some(&0) {
        little.pop();
    }
    little.iter().rev().copied().collect()
}
