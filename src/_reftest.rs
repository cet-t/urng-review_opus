#![allow(dead_code, clippy::all)]
//! Independent reference implementations written from the published specs
//! (Matsumoto/Nishimura MT, Saito/Matsumoto SFMT, Random123, Skein 1.3,
//! RFC 8439, Bernstein Salsa20, Vigna xoshiro/xoroshiro, PractRand SFC/JSF,
//! O'Neill PCG). Used only to cross-check urng outputs.

pub fn mt_init(seed: u32) -> [u32; 624] {
    let mut mt = [0u32; 624];
    mt[0] = seed;
    for i in 1..624 {
        mt[i] = 1812433253u32
            .wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30))
            .wrapping_add(i as u32);
    }
    mt
}

pub fn mt64_init(seed: u64) -> [u64; 312] {
    let mut mt = [0u64; 312];
    mt[0] = seed;
    for i in 1..312 {
        mt[i] = 6364136223846793005u64
            .wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 62))
            .wrapping_add(i as u64);
    }
    mt
}

/// sfmt_init_gen_rand for MEXP=19937 (incl. period certification).
pub fn sfmt19937_init(seed: u32) -> [u32; 624] {
    let mut p = [0u32; 624];
    p[0] = seed;
    for i in 1..624 {
        p[i] = 1812433253u32
            .wrapping_mul(p[i - 1] ^ (p[i - 1] >> 30))
            .wrapping_add(i as u32);
    }
    let parity = [0x00000001u32, 0x00000000, 0x00000000, 0x13c9e684];
    let mut inner = 0u32;
    for i in 0..4 {
        inner ^= p[i] & parity[i];
    }
    let mut i = 16;
    while i > 0 {
        inner ^= inner >> i;
        i >>= 1;
    }
    inner &= 1;
    if inner != 1 {
        'outer: for i in 0..4 {
            let mut work = 1u32;
            for _ in 0..32 {
                if work & parity[i] != 0 {
                    p[i] ^= work;
                    break 'outer;
                }
                work <<= 1;
            }
        }
    }
    p
}

pub fn sfc32(s: &mut [u32; 4]) -> u32 {
    let [a, b, c, ctr] = *s;
    let tmp = a.wrapping_add(b).wrapping_add(ctr);
    *s = [
        b ^ (b >> 9),
        c.wrapping_add(c << 3),
        c.rotate_left(21).wrapping_add(tmp),
        ctr.wrapping_add(1),
    ];
    tmp
}

/// PractRand sfc64: c = rotl(c, 24) + tmp
pub fn sfc64(s: &mut [u64; 4]) -> u64 {
    let [a, b, c, ctr] = *s;
    let tmp = a.wrapping_add(b).wrapping_add(ctr);
    *s = [
        b ^ (b >> 11),
        c.wrapping_add(c << 3),
        c.rotate_left(24).wrapping_add(tmp),
        ctr.wrapping_add(1),
    ];
    tmp
}

pub fn jsf32(s: &mut [u32; 4]) -> u32 {
    let [a, b, c, d] = *s;
    let e = a.wrapping_sub(b.rotate_left(27));
    let a2 = b ^ c.rotate_left(17);
    let b2 = c.wrapping_add(d);
    let c2 = d.wrapping_add(e);
    let d2 = e.wrapping_add(a2);
    *s = [a2, b2, c2, d2];
    d2
}

/// PCG-XSH-RR; `inc` is used as-is (caller decides whether it is odd).
pub fn pcg32_raw(state: &mut u64, inc: u64) -> u32 {
    let old = *state;
    *state = old.wrapping_mul(6364136223846793005).wrapping_add(inc);
    let xs = (((old >> 18) ^ old) >> 27) as u32;
    let rot = (old >> 59) as u32;
    xs.rotate_right(rot)
}

pub fn xoshiro256pp(s: &mut [u64; 4]) -> u64 {
    let r = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
    let t = s[1] << 17;
    s[2] ^= s[0];
    s[3] ^= s[1];
    s[1] ^= s[2];
    s[0] ^= s[3];
    s[2] ^= t;
    s[3] = s[3].rotate_left(45);
    r
}

pub fn xoshiro256ss(s: &mut [u64; 4]) -> u64 {
    let r = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
    let t = s[1] << 17;
    s[2] ^= s[0];
    s[3] ^= s[1];
    s[1] ^= s[2];
    s[0] ^= s[3];
    s[2] ^= t;
    s[3] = s[3].rotate_left(45);
    r
}

pub fn xoshiro128pp(s: &mut [u32; 4]) -> u32 {
    let r = s[0].wrapping_add(s[3]).rotate_left(7).wrapping_add(s[0]);
    let t = s[1] << 9;
    s[2] ^= s[0];
    s[3] ^= s[1];
    s[1] ^= s[2];
    s[0] ^= s[3];
    s[2] ^= t;
    s[3] = s[3].rotate_left(11);
    r
}

pub fn xoshiro128ss(s: &mut [u32; 4]) -> u32 {
    let r = s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
    let t = s[1] << 9;
    s[2] ^= s[0];
    s[3] ^= s[1];
    s[1] ^= s[2];
    s[0] ^= s[3];
    s[2] ^= t;
    s[3] = s[3].rotate_left(11);
    r
}

pub fn xoroshiro64ss(s: &mut [u32; 2]) -> u32 {
    let s0 = s[0];
    let mut s1 = s[1];
    let r = s0.wrapping_mul(0x9E3779BB).rotate_left(5).wrapping_mul(5);
    s1 ^= s0;
    s[0] = s0.rotate_left(26) ^ s1 ^ (s1 << 9);
    s[1] = s1.rotate_left(13);
    r
}

/// xoroshiro128++ (Vigna 2019): engine (49, 21, 28)
pub fn xoroshiro128pp(s: &mut [u64; 2]) -> u64 {
    let s0 = s[0];
    let mut s1 = s[1];
    let r = s0.wrapping_add(s1).rotate_left(17).wrapping_add(s0);
    s1 ^= s0;
    s[0] = s0.rotate_left(49) ^ s1 ^ (s1 << 21);
    s[1] = s1.rotate_left(28);
    r
}

/// xoroshiro128** : engine (24, 16, 37)
pub fn xoroshiro128ss(s: &mut [u64; 2]) -> u64 {
    let s0 = s[0];
    let mut s1 = s[1];
    let r = s0.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
    s1 ^= s0;
    s[0] = s0.rotate_left(24) ^ s1 ^ (s1 << 16);
    s[1] = s1.rotate_left(37);
    r
}

/// xoroshiro128+ (2018 constants): engine (24, 16, 37), output s0 + s1
pub fn xoroshiro128p(s: &mut [u64; 2]) -> u64 {
    let s0 = s[0];
    let mut s1 = s[1];
    let r = s0.wrapping_add(s1);
    s1 ^= s0;
    s[0] = s0.rotate_left(24) ^ s1 ^ (s1 << 16);
    s[1] = s1.rotate_left(37);
    r
}

/// Vigna reference jump()/long_jump() for xoroshiro128 engines.
pub fn xoroshiro128_jump(s: &mut [u64; 2], table: [u64; 2], step: fn(&mut [u64; 2]) -> u64) {
    let (mut s0, mut s1) = (0u64, 0u64);
    for w in table {
        for b in 0..64 {
            if w & (1u64 << b) != 0 {
                s0 ^= s[0];
                s1 ^= s[1];
            }
            step(s);
        }
    }
    *s = [s0, s1];
}

pub fn xorshift128(x: &mut [u32; 4]) -> u32 {
    let mut t = x[3];
    let s = x[0];
    x[3] = x[2];
    x[2] = x[1];
    x[1] = s;
    t ^= t << 11;
    t ^= t >> 8;
    x[0] = t ^ s ^ (s >> 19);
    x[0]
}

pub fn xorwow(x: &mut [u32; 5], c: &mut u32) -> u32 {
    let mut t = x[4];
    let s = x[0];
    x[4] = x[3];
    x[3] = x[2];
    x[2] = x[1];
    x[1] = s;
    t ^= t >> 2;
    t ^= t << 1;
    t ^= s ^ (s << 4);
    x[0] = t;
    *c = c.wrapping_add(362437);
    t.wrapping_add(*c)
}

pub fn wyhash64(s: &mut u64) -> u64 {
    *s = s.wrapping_add(0x60bee2bee120fc15);
    let t = (*s as u128) * 0xa3b195354a39b70d;
    let m1 = ((t >> 64) ^ t) as u64;
    let t = (m1 as u128) * 0x1b03738712fad5c9;
    ((t >> 64) ^ t) as u64
}

pub fn splitmix64(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

// ---------------- Random123 ----------------

pub fn philox4x32_10(ctr: [u32; 4], key: [u32; 2]) -> [u32; 4] {
    let mut c = ctr;
    let mut k = key;
    for r in 0..10 {
        if r > 0 {
            k[0] = k[0].wrapping_add(0x9E3779B9);
            k[1] = k[1].wrapping_add(0xBB67AE85);
        }
        let p0 = (c[0] as u64) * 0xD2511F53u64;
        let p1 = (c[2] as u64) * 0xCD9E8D57u64;
        c = [
            ((p1 >> 32) as u32) ^ c[1] ^ k[0],
            p1 as u32,
            ((p0 >> 32) as u32) ^ c[3] ^ k[1],
            p0 as u32,
        ];
    }
    c
}

pub fn philox2x64_10(ctr: [u64; 2], key: u64) -> [u64; 2] {
    let mut c = ctr;
    let mut k = key;
    for r in 0..10 {
        if r > 0 {
            k = k.wrapping_add(0x9E3779B97F4A7C15);
        }
        let p = (c[0] as u128) * 0xD2B74407B1CE6E93u128;
        c = [((p >> 64) as u64) ^ k ^ c[1], p as u64];
    }
    c
}

pub fn philox4x64_10(ctr: [u64; 4], key: [u64; 2]) -> [u64; 4] {
    let mut c = ctr;
    let mut k = key;
    for r in 0..10 {
        if r > 0 {
            k[0] = k[0].wrapping_add(0x9E3779B97F4A7C15);
            k[1] = k[1].wrapping_add(0xBB67AE8584CAA73B);
        }
        let p0 = (c[0] as u128) * 0xD2E7470EE14C6C93u128;
        let p1 = (c[2] as u128) * 0xCA5A826395121157u128;
        c = [
            ((p1 >> 64) as u64) ^ c[1] ^ k[0],
            p1 as u64,
            ((p0 >> 64) as u64) ^ c[3] ^ k[1],
            p0 as u64,
        ];
    }
    c
}

pub fn threefry2x32_20(ctr: [u32; 2], key: [u32; 2]) -> [u32; 2] {
    const R: [u32; 8] = [13, 15, 26, 6, 17, 29, 16, 24];
    let ks = [key[0], key[1], 0x1BD11BDA ^ key[0] ^ key[1]];
    let mut x = [ctr[0].wrapping_add(ks[0]), ctr[1].wrapping_add(ks[1])];
    for r in 0..20 {
        x[0] = x[0].wrapping_add(x[1]);
        x[1] = x[1].rotate_left(R[r % 8]);
        x[1] ^= x[0];
        if r % 4 == 3 {
            let s = r / 4 + 1;
            x[0] = x[0].wrapping_add(ks[s % 3]);
            x[1] = x[1].wrapping_add(ks[(s + 1) % 3]).wrapping_add(s as u32);
        }
    }
    x
}

pub fn threefry4x32_20(ctr: [u32; 4], key: [u32; 4]) -> [u32; 4] {
    const R: [[u32; 2]; 8] = [
        [10, 26],
        [11, 21],
        [13, 27],
        [23, 5],
        [6, 20],
        [17, 11],
        [25, 10],
        [18, 20],
    ];
    let ks = [
        key[0],
        key[1],
        key[2],
        key[3],
        0x1BD11BDA ^ key[0] ^ key[1] ^ key[2] ^ key[3],
    ];
    let mut x = [
        ctr[0].wrapping_add(ks[0]),
        ctr[1].wrapping_add(ks[1]),
        ctr[2].wrapping_add(ks[2]),
        ctr[3].wrapping_add(ks[3]),
    ];
    for r in 0..20 {
        let rr = R[r % 8];
        if r % 2 == 0 {
            x[0] = x[0].wrapping_add(x[1]);
            x[1] = x[1].rotate_left(rr[0]);
            x[1] ^= x[0];
            x[2] = x[2].wrapping_add(x[3]);
            x[3] = x[3].rotate_left(rr[1]);
            x[3] ^= x[2];
        } else {
            x[0] = x[0].wrapping_add(x[3]);
            x[3] = x[3].rotate_left(rr[0]);
            x[3] ^= x[0];
            x[2] = x[2].wrapping_add(x[1]);
            x[1] = x[1].rotate_left(rr[1]);
            x[1] ^= x[2];
        }
        if r % 4 == 3 {
            let s = r / 4 + 1;
            for i in 0..4 {
                x[i] = x[i].wrapping_add(ks[(s + i) % 5]);
            }
            x[3] = x[3].wrapping_add(s as u32);
        }
    }
    x
}

/// Threefish-256 encryption (Skein 1.3), 72 rounds.
pub fn threefish256(key: [u64; 4], tweak: [u64; 2], pt: [u64; 4]) -> [u64; 4] {
    const R: [[u32; 2]; 8] = [
        [14, 16],
        [52, 57],
        [23, 40],
        [5, 37],
        [25, 33],
        [46, 12],
        [58, 22],
        [32, 32],
    ];
    let k4 = 0x1BD11BDAA9FC1A22 ^ key[0] ^ key[1] ^ key[2] ^ key[3];
    let ks = [key[0], key[1], key[2], key[3], k4];
    let t = [tweak[0], tweak[1], tweak[0] ^ tweak[1]];
    let subkey = |s: usize| -> [u64; 4] {
        [
            ks[s % 5],
            ks[(s + 1) % 5].wrapping_add(t[s % 3]),
            ks[(s + 2) % 5].wrapping_add(t[(s + 1) % 3]),
            ks[(s + 3) % 5].wrapping_add(s as u64),
        ]
    };
    let mut v = pt;
    for d in 0..72 {
        let e = if d % 4 == 0 {
            let sk = subkey(d / 4);
            [
                v[0].wrapping_add(sk[0]),
                v[1].wrapping_add(sk[1]),
                v[2].wrapping_add(sk[2]),
                v[3].wrapping_add(sk[3]),
            ]
        } else {
            v
        };
        let r = R[d % 8];
        let f0 = e[0].wrapping_add(e[1]);
        let f1 = e[1].rotate_left(r[0]) ^ f0;
        let f2 = e[2].wrapping_add(e[3]);
        let f3 = e[3].rotate_left(r[1]) ^ f2;
        v = [f0, f3, f2, f1];
    }
    let sk = subkey(18);
    [
        v[0].wrapping_add(sk[0]),
        v[1].wrapping_add(sk[1]),
        v[2].wrapping_add(sk[2]),
        v[3].wrapping_add(sk[3]),
    ]
}

// ---------------- ChaCha / Salsa ----------------

pub fn chacha_double_round(x: &mut [u32; 16]) {
    fn qr(x: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
        x[a] = x[a].wrapping_add(x[b]);
        x[d] ^= x[a];
        x[d] = x[d].rotate_left(16);
        x[c] = x[c].wrapping_add(x[d]);
        x[b] ^= x[c];
        x[b] = x[b].rotate_left(12);
        x[a] = x[a].wrapping_add(x[b]);
        x[d] ^= x[a];
        x[d] = x[d].rotate_left(8);
        x[c] = x[c].wrapping_add(x[d]);
        x[b] ^= x[c];
        x[b] = x[b].rotate_left(7);
    }
    qr(x, 0, 4, 8, 12);
    qr(x, 1, 5, 9, 13);
    qr(x, 2, 6, 10, 14);
    qr(x, 3, 7, 11, 15);
    qr(x, 0, 5, 10, 15);
    qr(x, 1, 6, 11, 12);
    qr(x, 2, 7, 8, 13);
    qr(x, 3, 4, 9, 14);
}

/// RFC 8439 block function: 10 double rounds + feed-forward.
pub fn chacha20_block(input: &[u32; 16]) -> [u32; 16] {
    let mut x = *input;
    for _ in 0..10 {
        chacha_double_round(&mut x);
    }
    for i in 0..16 {
        x[i] = x[i].wrapping_add(input[i]);
    }
    x
}

pub fn salsa_qr(y: [u32; 4], r3: u32) -> [u32; 4] {
    let [mut a, mut b, mut c, mut d] = y;
    b ^= a.wrapping_add(d).rotate_left(7);
    c ^= b.wrapping_add(a).rotate_left(9);
    d ^= c.wrapping_add(b).rotate_left(r3);
    a ^= d.wrapping_add(c).rotate_left(18);
    [a, b, c, d]
}

/// One Salsa double round (column + row); `r3` is the 3rd rotation (spec: 13).
pub fn salsa_double_round(x: &mut [u32; 16], r3: u32) {
    let idx: [[usize; 4]; 8] = [
        [0, 4, 8, 12],
        [5, 9, 13, 1],
        [10, 14, 2, 6],
        [15, 3, 7, 11],
        [0, 1, 2, 3],
        [5, 6, 7, 4],
        [10, 11, 8, 9],
        [15, 12, 13, 14],
    ];
    for q in idx {
        let y = salsa_qr([x[q[0]], x[q[1]], x[q[2]], x[q[3]]], r3);
        for i in 0..4 {
            x[q[i]] = y[i];
        }
    }
}

// ---------------- LCG jump-ahead (Brown 1994) ----------------

pub fn lcg_advance(state: u64, mult: u64, inc: u64, mut delta: u64) -> u64 {
    let (mut acc_mult, mut acc_plus) = (1u64, 0u64);
    let (mut cur_mult, mut cur_plus) = (mult, inc);
    while delta > 0 {
        if delta & 1 == 1 {
            acc_mult = acc_mult.wrapping_mul(cur_mult);
            acc_plus = acc_plus.wrapping_mul(cur_mult).wrapping_add(cur_plus);
        }
        cur_plus = cur_mult.wrapping_add(1).wrapping_mul(cur_plus);
        cur_mult = cur_mult.wrapping_mul(cur_mult);
        delta >>= 1;
    }
    acc_mult.wrapping_mul(state).wrapping_add(acc_plus)
}

// ---------------- GF(2) 128x128 matrices ----------------
pub mod gf2 {
    /// Column representation: m[j] = T(e_j).
    pub type M = Vec<u128>;

    pub fn apply(m: &[u128], x: u128) -> u128 {
        let mut r = 0u128;
        let mut v = x;
        while v != 0 {
            let j = v.trailing_zeros() as usize;
            r ^= m[j];
            v &= v - 1;
        }
        r
    }
    pub fn from_fn(f: impl Fn(u128) -> u128) -> M {
        (0..128).map(|j| f(1u128 << j)).collect()
    }
    pub fn mul(a: &M, b: &M) -> M {
        b.iter().map(|&c| apply(a, c)).collect()
    }
    pub fn ident() -> M {
        (0..128).map(|j| 1u128 << j).collect()
    }
    pub fn is_ident(m: &M) -> bool {
        m.iter().enumerate().all(|(j, &c)| c == 1u128 << j)
    }
    pub fn pow(m: &M, mut e: u128) -> M {
        let mut base = m.clone();
        let mut acc = ident();
        while e > 0 {
            if e & 1 == 1 {
                acc = mul(&base, &acc);
            }
            base = mul(&base, &base);
            e >>= 1;
        }
        acc
    }
    pub fn pow2(m: &M, k: u32) -> M {
        let mut r = m.clone();
        for _ in 0..k {
            r = mul(&r, &r);
        }
        r
    }
    pub fn rank(m: &M) -> usize {
        let mut rows = m.clone();
        let mut rank = 0;
        for bit in 0..128 {
            let mask = 1u128 << bit;
            if let Some(p) = (rank..rows.len()).find(|&i| rows[i] & mask != 0) {
                rows.swap(rank, p);
                let pivot = rows[rank];
                for i in 0..rows.len() {
                    if i != rank && rows[i] & mask != 0 {
                        rows[i] ^= pivot;
                    }
                }
                rank += 1;
            }
        }
        rank
    }
    pub const FACTORS_2_128_M1: [u128; 9] = [
        3,
        5,
        17,
        257,
        641,
        65537,
        274177,
        6700417,
        67280421310721,
    ];
    /// true iff ord(T) == 2^128-1 (maximal period for every nonzero state)
    pub fn has_max_order(t: &M) -> bool {
        let n = u128::MAX;
        if !is_ident(&pow(t, n)) {
            return false;
        }
        FACTORS_2_128_M1.iter().all(|&p| !is_ident(&pow(t, n / p)))
    }
}
