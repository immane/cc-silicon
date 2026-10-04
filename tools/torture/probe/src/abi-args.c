/*
 * H01 reference-only target probe: AAPCS64 argument-classification exercise.
 *
 * This source is compiled (and, for the behavioural checks, executed) ONLY by
 * the explicitly pinned reference GCC on the frozen AArch64 GNU/Linux
 * substrate. It is an H01 host artifact, not part of the framework/compiler.
 *
 * Purpose: force the reference compiler to make the argument/return
 * classification decisions that T01 must freeze, and capture both:
 *   - runtime behaviour (`abi.*.ok` self-checks below), and
 *   - the emitted assembly (`gcc -S`, kept as evidence by run-probe.sh).
 *
 * Honest scope: C cannot observe how many argument registers the ABI defines
 * without inspecting emitted code. The numeric AAPCS64 register counts
 * (`abi.gp_arg_regs`, `abi.fp_arg_regs`) and `abi.variadic_register_save_area`
 * are therefore NOT guessed here; the assembly above is the evidence a later
 * classifier/integration review must interpret. The scalar values in
 * scalar-layout.c are directly measured and carry no such caveat.
 *
 * It prints deterministic `key=value` lines only: no timestamps, addresses,
 * host names, or locale-dependent floating formatting.
 */

#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* Homogeneous floating-point aggregate (HFA). */
typedef struct {
    float a, b, c, d;
} hfa_4f;

typedef struct {
    double a, b, c, d;
} hfa_4d;

/* Aggregate with one GP and one FP member: split classification. */
typedef struct {
    long i;
    double d;
} split_gp_fp;

/* Aggregate larger than the register file can hold: passed by reference. */
typedef struct {
    unsigned char raw[32];
} big_32;

/*
 * Ten GP arguments and ten FP arguments. On AAPCS64 the last members of each
 * list overflow to the stack; the emitted assembly is the classification
 * evidence. The reference compiler is the only compiler that ever sees this.
 */
long abi_gp_ten(long a, long b, long c, long d, long e,
                long f, long g, long h, long i, long j) {
    return a + b + c + d + e + f + g + h + i + j;
}

double abi_fp_ten(double a, double b, double c, double d, double e,
                  double f, double g, double h, double i, double j) {
    return a + b + c + d + e + f + g + h + i + j;
}

/* Aggregate returns: HFA in FP registers, split in GP+FP, large indirectly. */
hfa_4f abi_ret_hfa_4f(hfa_4f s) { return s; }
hfa_4d abi_ret_hfa_4d(hfa_4d s) { return s; }
split_gp_fp abi_ret_split(split_gp_fp s) { return s; }
big_32 abi_ret_big(big_32 s) { return s; }

/* Interleaved GP/FP scalar arguments. */
double abi_mixed_many(long i0, double d0, long i1, double d1, long i2,
                      double d2, long i3, double d3, long i4, double d4) {
    return (double)(i0 + i1 + i2 + i3 + i4) + d0 + d1 + d2 + d3 + d4;
}

/*
 * Variadic callee reading a fixed GP/FP pattern. On AAPCS64 this compels the
 * standard register save area; the emitted assembly is the evidence.
 * Callers below must match the read types exactly.
 */
long abi_variadic_sum(int n, ...) {
    va_list ap;
    long total = 0;
    long fptotal = 0;
    int i;
    va_start(ap, n);
    for (i = 0; i < n; ++i) {
        int kind = i % 3;
        if (kind == 0) {
            total += (long)va_arg(ap, int);
        } else if (kind == 1) {
            fptotal += (long)va_arg(ap, double);
        } else {
            total += va_arg(ap, long);
        }
    }
    va_end(ap);
    return total + fptotal;
}

int main(void) {
    hfa_4f hf4 = {1.0f, 2.0f, 3.0f, 4.0f};
    hfa_4d hd4 = {1.0, 2.0, 3.0, 4.0};
    split_gp_fp split = {7, 2.5};
    big_32 big;
    big_32 got;
    hfa_4f rf;
    hfa_4d rd;
    split_gp_fp rs;
    size_t k;

    long gp = abi_gp_ten(0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
    double fp = abi_fp_ten(0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
    double mixed = abi_mixed_many(1, 0.5, 2, 1.5, 3, 2.5, 4, 3.5, 5, 4.5);
    long var = abi_variadic_sum(6, 10, 0.5, 20L, 30, 1.5, 40L);

    for (k = 0; k < sizeof big.raw; ++k) {
        big.raw[k] = (unsigned char)k;
    }

    rf = abi_ret_hfa_4f(hf4);
    rd = abi_ret_hfa_4d(hd4);
    rs = abi_ret_split(split);
    got = abi_ret_big(big);

    /* Aggregate layout facts used to interpret the classification. */
    printf("abi.sizeof.hfa_4f=%zu\n", sizeof(hfa_4f));
    printf("abi.alignof.hfa_4f=%zu\n", __alignof__(hfa_4f));
    printf("abi.sizeof.hfa_4d=%zu\n", sizeof(hfa_4d));
    printf("abi.alignof.hfa_4d=%zu\n", __alignof__(hfa_4d));
    printf("abi.sizeof.split_gp_fp=%zu\n", sizeof(split_gp_fp));
    printf("abi.alignof.split_gp_fp=%zu\n", __alignof__(split_gp_fp));
    printf("abi.sizeof.big_32=%zu\n", sizeof(big_32));
    printf("abi.sizeof.va_list=%zu\n", sizeof(va_list));
    printf("abi.alignof.va_list=%zu\n", (size_t)__alignof__(va_list));

    /* Behavioural self-checks. These prove the reference ABI round-trips the
     * values; they do not by themselves count registers. */
    printf("abi.gp_ten.ok=%d\n", gp == 45);
    printf("abi.fp_ten.ok=%d\n", fp == 45.0);
    printf("abi.mixed_many.ok=%d\n", mixed == 27.5);
    printf("abi.variadic.ok=%d\n", var == 101);
    printf("abi.ret_hfa_4f.ok=%d\n",
           (rf.a == 1.0f && rf.b == 2.0f && rf.c == 3.0f && rf.d == 4.0f));
    printf("abi.ret_hfa_4d.ok=%d\n",
           (rd.a == 1.0 && rd.b == 2.0 && rd.c == 3.0 && rd.d == 4.0));
    printf("abi.ret_split.ok=%d\n", (rs.i == 7 && rs.d == 2.5));
    printf("abi.ret_big.ok=%d\n", memcmp(got.raw, big.raw, sizeof big.raw) == 0);

    return 0;
}
