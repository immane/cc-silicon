// ============================================================================
// target.rs — target, dialect, and configuration schema (T01 C02)
//
// Target identity is FROZEN: AArch64 GNU/Linux, ELF, LP64, little-endian,
// AAPCS64. Every concrete numeric ABI value is a proposal until an attested
// probe report is supplied.
//
// Verification is intentionally hard to obtain. `VerificationState::Probed`
// cannot be constructed through the public API: it is reachable only through
// `TargetSpec::attest`, which validates a canonical, hash-signed probe report
// containing every required field. What is unforgeable is the *private state
// representation*; attestation validates caller-supplied report data and its
// hash, not the authenticity or physical provenance of a probe. A caller who
// can supply report bytes can still supply a fabricated report, and the report
// file is read at a point in time (TOCTOU). Trust in the report source is an
// integration responsibility.
//
// No macOS/Darwin ABI value may appear here. The AAPCS64 variadic model uses a
// register save area; the Darwin arm64 stack-only model is not represented.
// ============================================================================

use std::collections::BTreeMap;

use crate::codec::{hex32, sha256};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};

/// Byte order of the generated target.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Endianness {
    /// Least-significant byte first.
    Little,
    /// Most-significant byte first.
    Big,
}

impl Endianness {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Little => "little",
            Self::Big => "big",
        }
    }
}

/// Object/executable format of the generated target.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectFormat {
    /// ELF (Linux).
    Elf,
    /// Mach-O (Darwin) — not represented by this contract.
    MachO,
    /// COFF (Windows) — not represented by this contract.
    Coff,
}

impl ObjectFormat {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Elf => "elf",
            Self::MachO => "macho",
            Self::Coff => "coff",
        }
    }
}

/// C data model: the width relation between `int`, `long`, and pointers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DataModel {
    /// `long` = 8, `int` = 4, pointer = 8.
    Lp64,
    /// `long` = 4, `int` = 4, pointer = 8.
    Llp64,
    /// `long` = 4, `int` = 4, pointer = 4.
    Ilp32,
    /// `long` = 4, `int` = 4, pointer = 4, `long long` = 8.
    Lp32,
}

impl DataModel {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Lp64 => "lp64",
            Self::Llp64 => "llp64",
            Self::Ilp32 => "ilp32",
            Self::Lp32 => "lp32",
        }
    }
}

/// IEEE/ABI floating-point storage format.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FloatFormat {
    /// Binary16.
    IeeeBinary16,
    /// Binary32.
    IeeeBinary32,
    /// Binary64.
    IeeeBinary64,
    /// Binary128.
    IeeeBinary128,
    /// x87 80-bit extended.
    X87Extended,
}

impl FloatFormat {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::IeeeBinary16 => "ieee-binary16",
            Self::IeeeBinary32 => "ieee-binary32",
            Self::IeeeBinary64 => "ieee-binary64",
            Self::IeeeBinary128 => "ieee-binary128",
            Self::X87Extended => "x87-extended",
        }
    }

    /// Parse a probe report format label.
    pub fn parse(label: &str) -> Option<Self> {
        match label {
            "ieee-binary16" => Some(Self::IeeeBinary16),
            "ieee-binary32" => Some(Self::IeeeBinary32),
            "ieee-binary64" => Some(Self::IeeeBinary64),
            "ieee-binary128" => Some(Self::IeeeBinary128),
            "x87-extended" => Some(Self::X87Extended),
            _ => None,
        }
    }
}

/// Width, alignment, and signedness/format of one scalar type.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ScalarModel {
    /// Size in target bytes.
    pub size: u8,
    /// Alignment in target bytes.
    pub align: u8,
    /// Signedness for integer types (`None` for floating types).
    pub signed: Option<bool>,
    /// Floating format (`None` for integer types).
    pub float: Option<FloatFormat>,
}

impl ScalarModel {
    /// An integer model.
    pub const fn integer(size: u8, align: u8, signed: bool) -> Self {
        Self {
            size,
            align,
            signed: Some(signed),
            float: None,
        }
    }

    /// A floating model.
    pub const fn float(size: u8, align: u8, format: FloatFormat) -> Self {
        Self {
            size,
            align,
            signed: None,
            float: Some(format),
        }
    }
}

/// Every scalar type whose size/alignment the target model must pin down.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ScalarKind {
    /// `_Bool`.
    Bool,
    /// Plain `char`.
    Char,
    /// `signed char`.
    SChar,
    /// `unsigned char`.
    UChar,
    /// `short`.
    Short,
    /// `unsigned short`.
    UShort,
    /// `int`.
    Int,
    /// `unsigned int`.
    UInt,
    /// `long`.
    Long,
    /// `unsigned long`.
    ULong,
    /// `long long`.
    LongLong,
    /// `unsigned long long`.
    ULongLong,
    /// `__int128`.
    Int128,
    /// `unsigned __int128`.
    UInt128,
    /// `float`.
    Float,
    /// `double`.
    Double,
    /// `long double`.
    LongDouble,
    /// `void *`.
    Pointer,
    /// `size_t`.
    SizeT,
    /// `ptrdiff_t`.
    PtrDiffT,
    /// `wchar_t`.
    WCharT,
}

/// Number of scalar kinds tracked by [`TargetSpec`].
pub const SCALAR_COUNT: usize = 21;

impl ScalarKind {
    /// Every kind, in canonical order. Array indices are stable.
    pub const ALL: [ScalarKind; SCALAR_COUNT] = [
        ScalarKind::Bool,
        ScalarKind::Char,
        ScalarKind::SChar,
        ScalarKind::UChar,
        ScalarKind::Short,
        ScalarKind::UShort,
        ScalarKind::Int,
        ScalarKind::UInt,
        ScalarKind::Long,
        ScalarKind::ULong,
        ScalarKind::LongLong,
        ScalarKind::ULongLong,
        ScalarKind::Int128,
        ScalarKind::UInt128,
        ScalarKind::Float,
        ScalarKind::Double,
        ScalarKind::LongDouble,
        ScalarKind::Pointer,
        ScalarKind::SizeT,
        ScalarKind::PtrDiffT,
        ScalarKind::WCharT,
    ];

    /// Canonical index.
    pub const fn index(self) -> usize {
        match self {
            Self::Bool => 0,
            Self::Char => 1,
            Self::SChar => 2,
            Self::UChar => 3,
            Self::Short => 4,
            Self::UShort => 5,
            Self::Int => 6,
            Self::UInt => 7,
            Self::Long => 8,
            Self::ULong => 9,
            Self::LongLong => 10,
            Self::ULongLong => 11,
            Self::Int128 => 12,
            Self::UInt128 => 13,
            Self::Float => 14,
            Self::Double => 15,
            Self::LongDouble => 16,
            Self::Pointer => 17,
            Self::SizeT => 18,
            Self::PtrDiffT => 19,
            Self::WCharT => 20,
        }
    }

    /// Stable name used in diagnostics.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bool => "_Bool",
            Self::Char => "char",
            Self::SChar => "signed char",
            Self::UChar => "unsigned char",
            Self::Short => "short",
            Self::UShort => "unsigned short",
            Self::Int => "int",
            Self::UInt => "unsigned int",
            Self::Long => "long",
            Self::ULong => "unsigned long",
            Self::LongLong => "long long",
            Self::ULongLong => "unsigned long long",
            Self::Int128 => "__int128",
            Self::UInt128 => "unsigned __int128",
            Self::Float => "float",
            Self::Double => "double",
            Self::LongDouble => "long double",
            Self::Pointer => "void *",
            Self::SizeT => "size_t",
            Self::PtrDiffT => "ptrdiff_t",
            Self::WCharT => "wchar_t",
        }
    }

    /// Probe-report field segment for this kind (e.g. `sizeof.longlong`).
    pub const fn probe_name(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Char => "char",
            Self::SChar => "schar",
            Self::UChar => "uchar",
            Self::Short => "short",
            Self::UShort => "ushort",
            Self::Int => "int",
            Self::UInt => "uint",
            Self::Long => "long",
            Self::ULong => "ulong",
            Self::LongLong => "longlong",
            Self::ULongLong => "ulonglong",
            Self::Int128 => "__int128",
            Self::UInt128 => "unsigned __int128",
            Self::Float => "float",
            Self::Double => "double",
            Self::LongDouble => "longdouble",
            Self::Pointer => "pointer",
            Self::SizeT => "size_t",
            Self::PtrDiffT => "ptrdiff_t",
            Self::WCharT => "wchar_t",
        }
    }
}

/// Scalar kinds whose size/alignment the probe must actually measure.
pub const MEASURED_SCALARS: [ScalarKind; 13] = [
    ScalarKind::Char,
    ScalarKind::Short,
    ScalarKind::Int,
    ScalarKind::Long,
    ScalarKind::LongLong,
    ScalarKind::Int128,
    ScalarKind::Float,
    ScalarKind::Double,
    ScalarKind::LongDouble,
    ScalarKind::Pointer,
    ScalarKind::SizeT,
    ScalarKind::PtrDiffT,
    ScalarKind::WCharT,
];

/// AAPCS64 ABI parameters. Values are UNVERIFIED until a probe is attested.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AbiSpec {
    /// ABI name (frozen identity: AAPCS64).
    pub name: &'static str,
    /// Number of general-purpose argument registers.
    pub gp_arg_regs: u8,
    /// Number of floating-point argument registers.
    pub fp_arg_regs: u8,
    /// Stack alignment in bytes.
    pub stack_align: u8,
    /// Whether variadic arguments use the AArch64 register save area.
    ///
    /// Must be `true` on AAPCS64. The Darwin arm64 stack-only variadic model
    /// is not representable by this contract.
    pub variadic_register_save_area: bool,
}

/// Verification status of a target's concrete values.
///
/// The internal representation is private so callers cannot flip an
/// unverified target to verified by constructing the enum. Verification is
/// reachable only through [`TargetSpec::attest`].
///
/// Direct construction is rejected:
///
/// ```compile_fail
/// use cc_silicon_compiler::target::VerificationState;
/// let _ = VerificationState { inner: todo!() };
/// ```
///
/// The attestation constructor is crate-private:
///
/// ```compile_fail
/// use cc_silicon_compiler::target::VerificationState;
/// let _ = VerificationState::attested([0u8; 32]);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VerificationState {
    inner: VerificationStateInner,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum VerificationStateInner {
    Unverified,
    Probed { report_hash: [u8; 32] },
}

impl VerificationState {
    /// The unverified state.
    pub const UNVERIFIED: Self = Self {
        inner: VerificationStateInner::Unverified,
    };

    /// Whether the target may be used for code generation.
    pub const fn is_verified(self) -> bool {
        matches!(self.inner, VerificationStateInner::Probed { .. })
    }

    /// The attested report hash, when verified.
    pub const fn report_hash(self) -> Option<[u8; 32]> {
        match self.inner {
            VerificationStateInner::Probed { report_hash } => Some(report_hash),
            VerificationStateInner::Unverified => None,
        }
    }

    /// Attested state. Crate-private: external callers must use
    /// [`TargetSpec::attest`].
    pub(crate) const fn attested(report_hash: [u8; 32]) -> Self {
        Self {
            inner: VerificationStateInner::Probed { report_hash },
        }
    }
}

impl Default for VerificationState {
    fn default() -> Self {
        Self::UNVERIFIED
    }
}

/// Frozen target identity: the dimensions fixed by the T01 decision.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TargetProfile {
    /// Target triple (frozen: `aarch64-unknown-linux-gnu`).
    pub triple: &'static str,
    /// Object format (frozen: ELF).
    pub object_format: ObjectFormat,
    /// Data model (frozen: LP64).
    pub data_model: DataModel,
    /// Endianness (frozen: little).
    pub endianness: Endianness,
    /// ABI name (frozen: AAPCS64).
    pub abi_name: &'static str,
}

impl TargetProfile {
    /// The frozen T01 target identity: AArch64 GNU/Linux, ELF, LP64, LE, AAPCS64.
    pub const fn aarch64_unknown_linux_gnu() -> Self {
        Self {
            triple: "aarch64-unknown-linux-gnu",
            object_format: ObjectFormat::Elf,
            data_model: DataModel::Lp64,
            endianness: Endianness::Little,
            abi_name: "AAPCS64",
        }
    }
}

/// Planned environment that will run the target probe.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProbeSubstrate {
    /// Substrate kind (frozen plan: Linux CI/VM).
    pub kind: &'static str,
    /// Whether the substrate currently exists and is usable.
    pub available: bool,
    /// Human-readable notes.
    pub notes: &'static str,
}

/// Corpus acquisition policy declared by the T01 decision.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CorpusPolicy {
    /// How the corpus is obtained (frozen: on demand).
    pub fetch: &'static str,
    /// Whether fetched sources are hash-locked.
    pub hash_locked: bool,
    /// Whether a reference DejaGnu baseline is authorized at all.
    pub reference_baseline_authorized: bool,
    /// Whether a reference baseline currently exists.
    pub reference_baseline_available: bool,
    /// Whether reference output may count as candidate compiler evidence.
    pub reference_counts_as_candidate_evidence: bool,
}

impl CorpusPolicy {
    /// The frozen T01 policy.
    pub const fn t01() -> Self {
        Self {
            fetch: "on-demand",
            hash_locked: true,
            reference_baseline_authorized: true,
            reference_baseline_available: false,
            reference_counts_as_candidate_evidence: false,
        }
    }
}

/// Probe fields that must be measured before a target can be attested.
pub const REQUIRED_PROBE_FIELDS: &[&str] = &[
    "triple",
    "object_format",
    "endianness",
    "data_model",
    "sizeof.char",
    "alignof.char",
    "char.signed",
    "sizeof.short",
    "alignof.short",
    "sizeof.int",
    "alignof.int",
    "sizeof.long",
    "alignof.long",
    "sizeof.longlong",
    "alignof.longlong",
    "sizeof.__int128",
    "alignof.__int128",
    "sizeof.float",
    "alignof.float",
    "sizeof.double",
    "alignof.double",
    "sizeof.longdouble",
    "alignof.longdouble",
    "longdouble.format",
    "sizeof.pointer",
    "alignof.pointer",
    "sizeof.size_t",
    "alignof.size_t",
    "sizeof.ptrdiff_t",
    "alignof.ptrdiff_t",
    "sizeof.wchar_t",
    "alignof.wchar_t",
    "wchar_t.signed",
    "wchar_t.encoding",
    "abi.gp_arg_regs",
    "abi.fp_arg_regs",
    "abi.stack_align",
    "abi.variadic_register_save_area",
];

/// Wide character encoding of the target.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WideCharEncoding {
    /// UTF-32 (proposed for AArch64 GNU/Linux).
    Utf32,
    /// UTF-16.
    Utf16,
}

impl WideCharEncoding {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Utf32 => "utf32",
            Self::Utf16 => "utf16",
        }
    }

    /// Parse a probe report encoding label. Unknown encodings are rejected so
    /// a target is never attested with an unresolved wide-character model.
    pub fn parse(label: &str) -> Option<Self> {
        match label {
            "utf32" => Some(Self::Utf32),
            "utf16" => Some(Self::Utf16),
            _ => None,
        }
    }
}

/// The complete target model consumed by compile chips.
///
/// Fields are private so a target cannot be forged by struct literal; use
/// [`TargetSpec::aarch64_unknown_linux_gnu_unverified`] or
/// [`TargetSpec::attest`]. Accessors return copies of the small value types.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TargetSpec {
    profile: TargetProfile,
    verification: VerificationState,
    scalars: [ScalarModel; SCALAR_COUNT],
    abi: AbiSpec,
    wchar_encoding: WideCharEncoding,
}

impl TargetSpec {
    /// The frozen AArch64 GNU/Linux target with UNVERIFIED concrete values.
    ///
    /// The numbers are the expected AAPCS64/LP64 values, recorded only so a
    /// probe has something to compare against. They are explicitly not
    /// authoritative: `verification` is [`VerificationState::UNVERIFIED`].
    pub const fn aarch64_unknown_linux_gnu_unverified() -> Self {
        use FloatFormat::{IeeeBinary128, IeeeBinary32, IeeeBinary64};
        let mut scalars = [ScalarModel::integer(0, 0, false); SCALAR_COUNT];
        scalars[ScalarKind::Bool.index()] = ScalarModel::integer(1, 1, false);
        scalars[ScalarKind::Char.index()] = ScalarModel::integer(1, 1, false);
        scalars[ScalarKind::SChar.index()] = ScalarModel::integer(1, 1, true);
        scalars[ScalarKind::UChar.index()] = ScalarModel::integer(1, 1, false);
        scalars[ScalarKind::Short.index()] = ScalarModel::integer(2, 2, true);
        scalars[ScalarKind::UShort.index()] = ScalarModel::integer(2, 2, false);
        scalars[ScalarKind::Int.index()] = ScalarModel::integer(4, 4, true);
        scalars[ScalarKind::UInt.index()] = ScalarModel::integer(4, 4, false);
        scalars[ScalarKind::Long.index()] = ScalarModel::integer(8, 8, true);
        scalars[ScalarKind::ULong.index()] = ScalarModel::integer(8, 8, false);
        scalars[ScalarKind::LongLong.index()] = ScalarModel::integer(8, 8, true);
        scalars[ScalarKind::ULongLong.index()] = ScalarModel::integer(8, 8, false);
        scalars[ScalarKind::Int128.index()] = ScalarModel::integer(16, 16, true);
        scalars[ScalarKind::UInt128.index()] = ScalarModel::integer(16, 16, false);
        scalars[ScalarKind::Float.index()] = ScalarModel::float(4, 4, IeeeBinary32);
        scalars[ScalarKind::Double.index()] = ScalarModel::float(8, 8, IeeeBinary64);
        scalars[ScalarKind::LongDouble.index()] = ScalarModel::float(16, 16, IeeeBinary128);
        scalars[ScalarKind::Pointer.index()] = ScalarModel::integer(8, 8, false);
        scalars[ScalarKind::SizeT.index()] = ScalarModel::integer(8, 8, false);
        scalars[ScalarKind::PtrDiffT.index()] = ScalarModel::integer(8, 8, true);
        scalars[ScalarKind::WCharT.index()] = ScalarModel::integer(4, 4, true);
        Self {
            profile: TargetProfile::aarch64_unknown_linux_gnu(),
            verification: VerificationState::UNVERIFIED,
            scalars,
            abi: AbiSpec {
                name: "AAPCS64",
                gp_arg_regs: 8,
                fp_arg_regs: 8,
                stack_align: 16,
                variadic_register_save_area: true,
            },
            wchar_encoding: WideCharEncoding::Utf32,
        }
    }

    /// The frozen target identity.
    pub const fn profile(&self) -> TargetProfile {
        self.profile
    }

    /// The verification state.
    pub const fn verification(&self) -> VerificationState {
        self.verification
    }

    /// Look up one scalar model.
    pub const fn scalar(&self, kind: ScalarKind) -> ScalarModel {
        self.scalars[kind.index()]
    }

    /// ABI parameters.
    pub const fn abi(&self) -> AbiSpec {
        self.abi
    }

    /// Wide character encoding.
    pub const fn wchar_encoding(&self) -> WideCharEncoding {
        self.wchar_encoding
    }

    /// Require the target to be verified before codegen may consume it.
    pub fn require_verified(&self) -> Result<(), ConfigError> {
        if self.verification.is_verified() {
            Ok(())
        } else {
            Err(ConfigError::TargetUnverified {
                triple: self.profile.triple,
            })
        }
    }

    /// Attest a probe report and produce a verified target.
    ///
    /// Validation: the report must be marked verified, contain every
    /// [`REQUIRED_PROBE_FIELDS`] entry, match the frozen identity, and carry a
    /// `report_hash` equal to the SHA-256 of the canonical unsigned body. This
    /// validates report *data*; it does not prove that a physical probe ran.
    /// The caller is responsible for the trust and freshness (TOCTOU) of the
    /// report bytes it supplies.
    pub fn attest(report: &ProbeReport) -> Result<Self, ProbeError> {
        let report_hash = report.verify_verified()?;
        report.require_fields(REQUIRED_PROBE_FIELDS)?;
        let profile = TargetProfile::aarch64_unknown_linux_gnu();
        report.expect("triple", profile.triple)?;
        report.expect("object_format", profile.object_format.name())?;
        report.expect("endianness", profile.endianness.name())?;
        report.expect("data_model", profile.data_model.name())?;

        let mut target = Self::aarch64_unknown_linux_gnu_unverified();
        for kind in MEASURED_SCALARS {
            let size = report.u8_field(&format!("sizeof.{}", kind.probe_name()))?;
            let align = report.u8_field(&format!("alignof.{}", kind.probe_name()))?;
            target.scalars[kind.index()].size = size;
            target.scalars[kind.index()].align = align;
        }
        target.scalars[ScalarKind::Char.index()].signed = Some(report.bool_field("char.signed")?);
        target.scalars[ScalarKind::WCharT.index()].signed =
            Some(report.bool_field("wchar_t.signed")?);
        let encoding = report
            .get("wchar_t.encoding")
            .ok_or(ProbeError::MissingField {
                field: "wchar_t.encoding",
            })?;
        target.wchar_encoding =
            WideCharEncoding::parse(encoding).ok_or_else(|| ProbeError::BadValue {
                field: "wchar_t.encoding".to_owned(),
                value: encoding.to_owned(),
            })?;
        let format = report
            .get("longdouble.format")
            .ok_or(ProbeError::MissingField {
                field: "longdouble.format",
            })?;
        target.scalars[ScalarKind::LongDouble.index()].float = Some(
            FloatFormat::parse(format).ok_or_else(|| ProbeError::BadValue {
                field: "longdouble.format".to_owned(),
                value: format.to_owned(),
            })?,
        );

        target.abi = AbiSpec {
            name: "AAPCS64",
            gp_arg_regs: report.u8_field("abi.gp_arg_regs")?,
            fp_arg_regs: report.u8_field("abi.fp_arg_regs")?,
            stack_align: report.u8_field("abi.stack_align")?,
            variadic_register_save_area: report.bool_field("abi.variadic_register_save_area")?,
        };
        target.verification = VerificationState::attested(report_hash);
        Ok(target)
    }
}

/// Structured probe-report failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeError {
    /// A line was not `key = value`.
    MalformedLine {
        /// 1-based line number.
        line: usize,
    },
    /// A key appeared twice.
    DuplicateField {
        /// Offending field.
        field: String,
    },
    /// A required field is missing.
    MissingField {
        /// Offending field.
        field: &'static str,
    },
    /// A required dynamic field is missing.
    MissingFieldOwned {
        /// Offending field.
        field: String,
    },
    /// A field had an unparseable value.
    BadValue {
        /// Offending field.
        field: String,
        /// Observed value.
        value: String,
    },
    /// The report is not marked verified.
    NotVerified,
    /// The report hash is absent or malformed.
    BadHash,
    /// The report hash does not match the canonical body.
    HashMismatch {
        /// Declared hash.
        declared: String,
        /// Recomputed hash.
        computed: String,
    },
    /// A frozen identity field did not match.
    IdentityMismatch {
        /// Offending field.
        field: &'static str,
        /// Expected value.
        expected: String,
        /// Observed value.
        actual: String,
    },
}

impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedLine { line } => write!(f, "probe report line {line} is malformed"),
            Self::DuplicateField { field } => {
                write!(f, "probe report field `{field}` is duplicated")
            }
            Self::MissingField { field } => write!(f, "probe report is missing `{field}`"),
            Self::MissingFieldOwned { field } => write!(f, "probe report is missing `{field}`"),
            Self::BadValue { field, value } => {
                write!(f, "probe report field `{field}` has bad value `{value}`")
            }
            Self::NotVerified => write!(f, "probe report is not marked verified"),
            Self::BadHash => write!(f, "probe report hash is missing or malformed"),
            Self::HashMismatch { declared, computed } => write!(
                f,
                "probe report hash mismatch: declared {declared}, computed {computed}"
            ),
            Self::IdentityMismatch {
                field,
                expected,
                actual,
            } => write!(
                f,
                "probe report `{field}` is `{actual}`, expected `{expected}`"
            ),
        }
    }
}

impl std::error::Error for ProbeError {}

/// Machine-readable status parsed from a probe report.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProbeStatus {
    /// Not verified.
    Unverified,
    /// Verified with a report hash.
    Verified {
        /// Attested report hash.
        report_hash: [u8; 32],
    },
}

/// A parsed, canonical probe report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProbeReport {
    fields: BTreeMap<String, String>,
}

impl ProbeReport {
    /// Parse a report. Unknown keys are retained; comments and blank lines are
    /// ignored.
    pub fn parse(text: &str) -> Result<Self, ProbeError> {
        let mut fields = BTreeMap::new();
        for (index, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(ProbeError::MalformedLine { line: index + 1 });
            };
            let key = key.trim().to_owned();
            if key.is_empty() {
                return Err(ProbeError::MalformedLine { line: index + 1 });
            }
            if fields
                .insert(key.clone(), value.trim().to_owned())
                .is_some()
            {
                return Err(ProbeError::DuplicateField { field: key });
            }
        }
        Ok(Self { fields })
    }

    /// Read a field.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    /// Insert or replace a field.
    pub fn set(&mut self, key: &str, value: &str) {
        self.fields.insert(key.to_owned(), value.to_owned());
    }

    /// Canonical unsigned body: sorted `key=value\n` for every field except
    /// `report_hash`. This is the byte string the report hash covers.
    pub fn unsigned_body(&self) -> Vec<u8> {
        let mut body = Vec::new();
        for (key, value) in &self.fields {
            if key == "report_hash" {
                continue;
            }
            body.extend_from_slice(key.as_bytes());
            body.push(b'=');
            body.extend_from_slice(value.as_bytes());
            body.push(b'\n');
        }
        body
    }

    /// Verify the `verified` flag and, when true, the report hash.
    pub fn verify_verified(&self) -> Result<[u8; 32], ProbeError> {
        let verified = self.bool_field("verified")?;
        if !verified {
            return Err(ProbeError::NotVerified);
        }
        let declared = self.get("report_hash").ok_or(ProbeError::BadHash)?;
        if declared.len() != 64 || !declared.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ProbeError::BadHash);
        }
        let computed = sha256(&self.unsigned_body());
        if hex32(&computed) != declared.to_ascii_lowercase() {
            return Err(ProbeError::HashMismatch {
                declared: declared.to_owned(),
                computed: hex32(&computed),
            });
        }
        Ok(computed)
    }

    /// Machine-readable status without building a target.
    pub fn status(&self) -> Result<ProbeStatus, ProbeError> {
        let hash = self.verify_verified()?;
        self.require_fields(REQUIRED_PROBE_FIELDS)?;
        Ok(ProbeStatus::Verified { report_hash: hash })
    }

    /// Require a specific value for a field.
    pub fn expect(&self, key: &'static str, expected: &str) -> Result<(), ProbeError> {
        let actual = self
            .get(key)
            .ok_or(ProbeError::MissingField { field: key })?;
        if actual == expected {
            Ok(())
        } else {
            Err(ProbeError::IdentityMismatch {
                field: key,
                expected: expected.to_owned(),
                actual: actual.to_owned(),
            })
        }
    }

    /// Require every field in `required` to be present.
    pub fn require_fields(&self, required: &[&'static str]) -> Result<(), ProbeError> {
        for field in required {
            if self.get(field).is_none() {
                return Err(ProbeError::MissingField { field });
            }
        }
        Ok(())
    }

    /// Parse a `u8` field.
    pub fn u8_field(&self, key: &str) -> Result<u8, ProbeError> {
        let value = self.get(key).ok_or(ProbeError::MissingFieldOwned {
            field: key.to_owned(),
        })?;
        value.parse::<u8>().map_err(|_| ProbeError::BadValue {
            field: key.to_owned(),
            value: value.to_owned(),
        })
    }

    /// Parse a boolean field. Only `true`/`false` are accepted.
    pub fn bool_field(&self, key: &str) -> Result<bool, ProbeError> {
        let value = self.get(key).ok_or(ProbeError::MissingFieldOwned {
            field: key.to_owned(),
        })?;
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(ProbeError::BadValue {
                field: key.to_owned(),
                value: value.to_owned(),
            }),
        }
    }
}

/// C language dialect / standard mode.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Dialect {
    /// ISO C89 / C90.
    C89,
    /// ISO C99.
    C99,
    /// ISO C11.
    C11,
    /// ISO C17.
    C17,
    /// ISO C23.
    C23,
    /// GNU C89.
    Gnu89,
    /// GNU C99.
    Gnu99,
    /// GNU C11.
    Gnu11,
    /// GNU C17.
    Gnu17,
    /// GNU C23.
    Gnu23,
}

impl Dialect {
    /// Whether GNU extensions are enabled in this mode.
    pub const fn is_gnu(self) -> bool {
        matches!(
            self,
            Self::Gnu89 | Self::Gnu99 | Self::Gnu11 | Self::Gnu17 | Self::Gnu23
        )
    }

    /// The strict ISO dialect with the same base standard.
    pub const fn strict(self) -> Self {
        match self {
            Self::C89 | Self::Gnu89 => Self::C89,
            Self::C99 | Self::Gnu99 => Self::C99,
            Self::C11 | Self::Gnu11 => Self::C11,
            Self::C17 | Self::Gnu17 => Self::C17,
            Self::C23 | Self::Gnu23 => Self::C23,
        }
    }

    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::C89 => "c89",
            Self::C99 => "c99",
            Self::C11 => "c11",
            Self::C17 => "c17",
            Self::C23 => "c23",
            Self::Gnu89 => "gnu89",
            Self::Gnu99 => "gnu99",
            Self::Gnu11 => "gnu11",
            Self::Gnu17 => "gnu17",
            Self::Gnu23 => "gnu23",
        }
    }
}

/// Optimization level, mirroring the frozen option matrix.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum OptLevel {
    /// `-O0`.
    O0,
    /// `-O1`.
    O1,
    /// `-O2`.
    O2,
    /// `-O3`.
    O3,
    /// `-Os`.
    Os,
    /// `-Og`.
    Og,
}

impl OptLevel {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::O0 => "O0",
            Self::O1 => "O1",
            Self::O2 => "O2",
            Self::O3 => "O3",
            Self::Os => "Os",
            Self::Og => "Og",
        }
    }
}

/// Semantic option flags that change observable meaning and must be recorded.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum OptionFlag {
    /// `-fwrapv`: signed overflow wraps.
    Fwrapv,
    /// `-fno-strict-aliasing`.
    FnoStrictAliasing,
    /// `-fcommon`.
    Fcommon,
    /// `-fno-common`.
    FnoCommon,
    /// `-ffast-math`.
    FfastMath,
    /// `-fomit-frame-pointer`.
    FomitFramePointer,
    /// `-fpack-struct`.
    FpackStruct,
}

impl OptionFlag {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fwrapv => "fwrapv",
            Self::FnoStrictAliasing => "fno-strict-aliasing",
            Self::Fcommon => "fcommon",
            Self::FnoCommon => "fno-common",
            Self::FfastMath => "ffast-math",
            Self::FomitFramePointer => "fomit-frame-pointer",
            Self::FpackStruct => "fpack-struct",
        }
    }
}

/// Immutable job configuration: target, dialect, options, limits.
///
/// Fields are private; configuration is read-only after construction and
/// commit rejects any store patch targeting the `config` store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilerConfig {
    target: TargetSpec,
    dialect: Dialect,
    opt_level: OptLevel,
    options: Vec<OptionFlag>,
    limits: crate::limits::Limits,
}

/// Structured configuration failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Codegen was requested while the target values are unverified.
    TargetUnverified {
        /// Target triple.
        triple: &'static str,
    },
    /// The same semantic option was requested twice.
    DuplicateOption {
        /// Offending option.
        option: OptionFlag,
    },
}

impl ConfigError {
    /// Map to a structured diagnostic.
    pub fn to_diagnostic(&self) -> DiagnosticDraft {
        match self {
            Self::TargetUnverified { triple } => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Target, 1),
                format!("target `{triple}` concrete values are unverified; run and attest the Linux probe first"),
            ),
            Self::DuplicateOption { option } => DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Config, 1),
                format!("option `{}` declared more than once", option.name()),
            ),
        }
    }
}

impl CompilerConfig {
    /// Build a configuration from its parts.
    pub fn new(
        target: TargetSpec,
        dialect: Dialect,
        opt_level: OptLevel,
        options: Vec<OptionFlag>,
        limits: crate::limits::Limits,
    ) -> Self {
        let mut config = Self {
            target,
            dialect,
            opt_level,
            options,
            limits,
        };
        config.canonicalize();
        config
    }

    /// Sort and deduplicate option flags so equal intent has one encoding.
    pub fn canonicalize(&mut self) {
        self.options.sort();
        self.options.dedup();
    }

    /// The target model.
    pub fn target(&self) -> &TargetSpec {
        &self.target
    }

    /// The language dialect.
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// The optimization level.
    pub fn opt_level(&self) -> OptLevel {
        self.opt_level
    }

    /// The canonical option flags.
    pub fn options(&self) -> &[OptionFlag] {
        &self.options
    }

    /// The resource limits.
    pub fn limits(&self) -> crate::limits::Limits {
        self.limits
    }

    /// Structural validation independent of target verification.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut sorted = self.options.clone();
        sorted.sort();
        for pair in sorted.windows(2) {
            if pair[0] == pair[1] {
                return Err(ConfigError::DuplicateOption { option: pair[0] });
            }
        }
        Ok(())
    }

    /// Require a verified target before code generation.
    pub fn ensure_codegen_ready(&self) -> Result<(), ConfigError> {
        self.target.require_verified()
    }
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self::new(
            TargetSpec::aarch64_unknown_linux_gnu_unverified(),
            Dialect::C11,
            OptLevel::O0,
            Vec::new(),
            crate::limits::Limits::default(),
        )
    }
}
