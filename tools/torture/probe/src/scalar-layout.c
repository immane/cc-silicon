/*
 * H01 reference-only target probe: scalar layout, encoding, and identity.
 *
 * This source is compiled and executed ONLY by the explicitly pinned reference
 * GCC on the frozen AArch64 GNU/Linux substrate (see ../run-probe.sh). It is a
 * host tool for H01 and is not part of the cc-silicon framework or compiler.
 *
 * Contract:
 *   - It measures facts; it does not assert target constants. Every numeric
 *     value in the report comes from the running toolchain.
 *   - It prints deterministic `key=value` lines on stdout and nothing else:
 *     no timestamps, no addresses, no user/host names, no locale-dependent
 *     floating formatting (only integers and lowercase hex).
 *   - It never invokes another compiler, never reads the network, and never
 *     reads a candidate binary.
 *
 * The normalizer (../normalize/normalize.py) sorts the keys, derives the small
 * set of classifications that are pure functions of the measured values
 * (endianness, data model, long double format, object format, wide-character
 * encoding), path-strips the report, and records the report SHA-256.
 */

#include <float.h>
#include <limits.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <wchar.h>

/* Largest representable `wchar_t`, from the compiler's own model where
 * available. Left undefined when neither source is known, so the UCN literal
 * below stays uncompiled rather than relying on an unknown range. */
#if defined(__WCHAR_MAX__)
#  define T00_WCHAR_MAX __WCHAR_MAX__
#elif defined(WCHAR_MAX)
#  define T00_WCHAR_MAX WCHAR_MAX
#endif

static void print_size_align(const char *tag, size_t size, size_t align) {
    printf("sizeof.%s=%zu\n", tag, size);
    printf("alignof.%s=%zu\n", tag, align);
}

static void print_hex(const char *tag, const void *value, size_t size) {
    const unsigned char *bytes = (const unsigned char *)value;
    size_t i;
    printf("encoding.%s=", tag);
    for (i = 0; i < size; ++i) {
        printf("%02x", (unsigned)bytes[i]);
    }
    printf("\n");
}

int main(void) {
    /* Character types. `char` signedness is measured, never assumed. */
    print_size_align("char", sizeof(char), _Alignof(char));
    print_size_align("signed_char", sizeof(signed char), _Alignof(signed char));
    print_size_align("unsigned_char", sizeof(unsigned char), _Alignof(unsigned char));
    printf("char.signed=%s\n", ((char)-1 < 0) ? "true" : "false");
    printf("char.bit=%d\n", CHAR_BIT);

    /* Boolean and integer widths. */
    print_size_align("_Bool", sizeof(_Bool), _Alignof(_Bool));
    print_size_align("short", sizeof(short), _Alignof(short));
    print_size_align("int", sizeof(int), _Alignof(int));
    print_size_align("long", sizeof(long), _Alignof(long));
    print_size_align("longlong", sizeof(long long), _Alignof(long long));

#ifdef __SIZEOF_INT128__
    print_size_align("__int128", sizeof(__int128), _Alignof(__int128));
    printf("int128.available=true\n");
#else
    printf("int128.available=false\n");
#endif

    /* Floating formats. Only exponents/mantissa are printed; the normalizer
     * maps them to the IEEE format name using documented rules. */
    print_size_align("float", sizeof(float), _Alignof(float));
    print_size_align("double", sizeof(double), _Alignof(double));
    print_size_align("longdouble", sizeof(long double), _Alignof(long double));

    printf("float.radix=%d\n", FLT_RADIX);
    printf("float.mant_dig=%d\n", FLT_MANT_DIG);
    printf("float.max_exp=%d\n", FLT_MAX_EXP);
    printf("float.min_exp=%d\n", FLT_MIN_EXP);
    printf("double.mant_dig=%d\n", DBL_MANT_DIG);
    printf("double.max_exp=%d\n", DBL_MAX_EXP);
    printf("double.min_exp=%d\n", DBL_MIN_EXP);
    printf("longdouble.radix=%d\n", FLT_RADIX);
    printf("longdouble.mant_dig=%d\n", LDBL_MANT_DIG);
    printf("longdouble.max_exp=%d\n", LDBL_MAX_EXP);
    printf("longdouble.min_exp=%d\n", LDBL_MIN_EXP);
    printf("longdouble.dig=%d\n", LDBL_DIG);

    {
        float f = 1.0f;
        double d = 1.0;
        long double ld = 1.0L;
        long double ld_neg = -2.0L;
        print_hex("float.1.0", &f, sizeof f);
        print_hex("double.1.0", &d, sizeof d);
        print_hex("longdouble.1.0", &ld, sizeof ld);
        print_hex("longdouble.-2.0", &ld_neg, sizeof ld_neg);
    }

    /* Endianness: a value whose bytes are distinguishable. */
    {
        uint32_t word = UINT32_C(0x01020304);
        print_hex("word.01020304", &word, sizeof word);
    }

    /* Pointer-like and library scalar types. */
    print_size_align("pointer", sizeof(void *), _Alignof(void *));
    print_size_align("size_t", sizeof(size_t), _Alignof(size_t));
    print_size_align("ptrdiff_t", sizeof(ptrdiff_t), _Alignof(ptrdiff_t));
    print_size_align("wchar_t", sizeof(wchar_t), _Alignof(wchar_t));
    printf("wchar_t.signed=%s\n", ((wchar_t)-1 < 0) ? "true" : "false");

    /* Wide-character encoding evidence. The encoding of `wchar_t` is
     * implementation-defined, and an integer cast like `(wchar_t)0x1F600`
     * only exercises integer range, not the wide-character literal encoding.
     * So this observes the implementation's actual wide literal:
     *   - whether the implementation claims ISO/IEC 10646 wchar_t semantics
     *     (`__STDC_ISO_10646__`);
     *   - whether the non-BMP wide character literal L'\U0001F600' is
     *     representable and stores U+1F600 exactly in one `wchar_t`;
     *   - the largest representable `wchar_t` value.
     * The normalizer labels the encoding only when these jointly justify it.
     *
     * The UCN literal is compiled only under a guard that requires both ISO
     * 10646 semantics and a range covering U+1F600, so a toolchain that cannot
     * represent the scalar never sees the literal and cannot fail to compile. */
    {
        int literal_available = 0;
        wchar_t literal_nonbmp = 0;
#if defined(__STDC_ISO_10646__) && defined(T00_WCHAR_MAX) && \
    (T00_WCHAR_MAX >= 0x1F600)
        literal_nonbmp = L'\U0001F600';
        literal_available = 1;
#endif
        printf("wchar_t.ucn_literal_available=%d\n", literal_available);
        if (literal_available) {
            printf("wchar_t.ucn_nonbmp_single=%d\n",
                   ((unsigned long)literal_nonbmp == 0x1F600UL) ? 1 : 0);
        } else {
            printf("wchar_t.ucn_nonbmp_single=absent\n");
        }
    }
#ifdef __STDC_ISO_10646__
    printf("wchar_t.stdc_iso_10646=%lld\n", (long long)__STDC_ISO_10646__);
#else
    printf("wchar_t.stdc_iso_10646=absent\n");
#endif
#if defined(__WCHAR_MAX__)
    printf("wchar_t.max=%lld\n", (long long)__WCHAR_MAX__);
#elif defined(WCHAR_MAX)
    printf("wchar_t.max=%lld\n", (long long)WCHAR_MAX);
#else
    printf("wchar_t.max=absent\n");
#endif

    print_size_align("max_align_t", sizeof(max_align_t), _Alignof(max_align_t));

    /* Compiler identity as the compiler itself defines it. */
    printf("compiler.__GNUC__=%d\n", __GNUC__);
    printf("compiler.__GNUC_MINOR__=%d\n", __GNUC_MINOR__);
    printf("compiler.__GNUC_PATCHLEVEL__=%d\n", __GNUC_PATCHLEVEL__);
#ifdef __STDC_VERSION__
    printf("compiler.__STDC_VERSION__=%ld\n", (long)__STDC_VERSION__);
#else
    printf("compiler.__STDC_VERSION__=absent\n");
#endif
#ifdef __VERSION__
    printf("compiler.__VERSION__=%s\n", __VERSION__);
#endif

    return 0;
}
