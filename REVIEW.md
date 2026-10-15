urng 1.0.0 コードレビュー

対象はこのリポジトリの作業コピーです。crates.io の urng-1.0.0 と内容が同一であること（改行コードの違いを除く）を確認しました。

A. 総評

設計の骨格は良くできています。Word/Rng トレイトで変換式を1か所に集めており、収録している生成器の種類も幅広いです。MT19937、SFMT、Philox2x64/4x64、Threefry2x32、Threefish-256 の本体、xoshiro/xoroshiro の主要な変種、SFC32・JSF32 などは、参照実装や公式テストベクタと照合して一致しました（D節）。

一方で、1.0.0 として公開するには重大な問題が多く残っています。

- ビルド: Linux など大文字小文字を区別するファイルシステムではコンパイルできません。docs.rs のビルドも失敗しています。
- シード: ほぼ全生成器でシード 2k と 2k+1 が同じ乱数列になります。
- アルゴリズム: 名前どおりの実装になっていない生成器が複数あります（Sfc64、Xoroshiro128Pp、ChaCha/Salsa、Philox32 など）。
- SIMD と C ABI: safe API からのメモリ破壊、「AVX2」と表記しながら AVX-512VL を要求する型、一括生成での大量重複があります。

既存のテストは「同じシードなら同じ出力になるか」しか見ていません。そのため、この環境（AVX-512 搭載機）では単体テスト 101件・doctest 104件がすべて通り、上記の欠陥を1件も検出できませんでした。

B. 発見した問題

B-1. 確認済みの欠陥

以下はすべて、実行またはソース・アセンブリで裏付

---

B1【Critical】大文字小文字を区別するファイルシステムでビルドできない

- 場所: src/lib.rs:1（include_str!("../Readme.md")）
- 問題: 実際のファイル名は README.md です。include_str! は通常のビルドでも評価されるため、Readme.md が見つからずコンパイルエラーになります。
- 発生条件・影響:
  - Linux（多くの CI を含む）、大文字小文字を区 rs で発生します。
  - 依存クレートとして追加した時点で cargo build が失敗します。
  - 現在 docs.rs にドキュメントが存在しません。
- 根拠:
  - docs.rs のビルド #4683312（x86_64-unknown-linux-gnu）は couldn't read `src/../Readme.md`: No such file or directory
    で失敗しています。
  - 手元でも、fsutil file setCaseSensitiveInfo で大文字小文字を区別するディレクトリを作り、公開パッケージを置いて cargo check すると同じエラーで失敗しました。
- 改善案: パスを ../README.md に直し、Linux の CI を追加します。
- 確信度: 高

---

B2【Critical】Xoshiro256Ssx2: safe API なのにスタック外へ書き込み、出力も0に収束する

  3. コードを実際に確認する。 READMEやファイル名だけから実装を推測しない。
  4. 指摘には根拠を添える。 ファイルパス、該当箇所、問題が起きる条件、影響を明示する。
  5. 好みと欠陥を区別する。 単なるスタイルの違いをバグとして扱わない。
  6. 勝手にコードを変更しない。 今回はレビューのみ。
  7. 調査できなかった範囲を明記する。 未確認のコードをレビュー済みと称さない。
  8. 検証したと偽らない。 実行していないテストやベンチマークの結果を捏造しない。

  出力形式

  A. 総評

  設計上の強み、懸念点、完成度について簡潔に評価する。

  B. 発見した問題

  各指摘を次の形式で報告する。

  - 重要度：Critical / High / Medium / Low
  - 場所：ファイルパスと行番号または関数名
  - 問題：具体的な説明
  - 発生条件・影響：どういう状況で何が起きるか
  - 根拠：該当コード、仕様、再現手順など
  - 改善案：最小限かつ現実的な修正方針
  - 確信度：高 / 中 / 低

  確認済みの欠陥と、追加検証が必要な懸念は分けること。

  C. 改善提案

  バグではないが改善価値のある設計・性能・保守性上 る。

  D. 評価できる設計

  技術的な根拠を示しながら、優れている実装や設計判断を挙げる。

  E. 検証状況と限界

  - 実際に読んだ範囲
  - 実行したテストやベンチマーク
  - 実行できなかった検証
  - 判断を保留した事項

  最後に

  このレビューはAI同士の比較に使用される。

  レビューの長さや指摘件数ではなく、正確性・発見の価値・根拠の強さで評価される。

  誤検知を増やしてまで指摘数を稼ぐ必要はない。

  作者にとって本当に役立つレビューを提出すること。

まずリポジトリの構造を把握します。

  Read 23 files, ran 2 shell commands

コアtrait・内部ヘルパ・各PRNG実装を精読し、シャッフルの偏り、PCGの増分、AVX2型でのAVX-512命令使用など要検証の疑いをいくつか記録しました。これらは後で実行して確認します。

  Read 13 files, ran 1 shell command

64bit系で重大な疑いを複数見つけました：Xoshiro256Ssx2の状態ゼロ化とスタック外書き込み、Xoroshiro128Ppの誤実装、SplitMixのseed | 1による隣接シード衝突です。残りを読んだ後、実際に実行して裏付けを取ります。

  Read 19 files

残りのseedgenとwideモジュール、C ABI層を読みます 用クレートを作り、疑わしい箇所を実行確認します（リポジトリ内のファイルは変更しません）。

  Searched for 9 patterns, read 39 files, ran 4 shell commands

● Fetch(https://docs.rs/crate/urng/1.0.0/builds)
Received 11.5KB (200 )

● Fetch(https://docs.rs/crate/urng/1.0.0/builds/4683312)
Received 14.5KB (200 )

  Searched for 2 patterns, read 3 files, ran 15 shell commands

I've confirmed the build-time issue: on a case-sensitive filesystem, published 1.0.0 fails cargo check with the same Readme.md error seen on docs.rs. Next I'll create a verification crate in the scratchpad that depends on a copy of the published crate (no build.rs, so this repo stays untouched) and test every suspected cause.

  Made 3 scratchpad edits +1791, searched for 1 pattern

Now let me fix the fn-pointer array typing defensively and build.

  Made 1 scratchpad edit +6 -5, ran 6 shell commands

ここまでで、隣接シードの完全一致（全27生成器）、シャッフルの偏り、TwistedGFSRのchoiceが常に0を返す件、PCGの周期短縮（2^45）、ChaCha20の次ブロック予測可能性を確認しました。次はSIMD版とC ABIの検証に進みます。

  Made 2 scratchpad edits +48 -9, ran 4 shell commands

SIMD/C ABIの検証が完了し、64bit SIMD一括生成では出力の大半が重複すること（sfc64x8_next_u64sで異なる値は約2.5%のみ）や、philox32x4x4_next_u32sの出力がバッファのアライメントで変化することを実測で確認しました。次はdoctest・ドキュメントビルド・rand連携のコンパイル確認とベンチマークの再現を行います。

  Ran 5 shell commands

aarch64環境でもクロスチェックを行い、simd/full featureがコンパイルできないことを確認しました。次はrand連携とREADMEサンプルのコンパイル確認、ベンチマークの再現を進めます。

  Made 2 scratchpad edits +57, searched for 1 pattern, read 2 files, ran 3 shell commands

● Fetch(https://raw.githubusercontent.com/MersenneTwister-Lab/SFMT/master/SFMT-params607.h)
Received 1.9KB (200 OK)

● Fetch(https://raw.githubusercontent.com/MersenneTwister-Lab/SFMT/master/SFMT-params1279.h)
Received 1.9KB (200 OK)

● Fetch(https://raw.githubusercontent.com/MersenneTwister-Lab/SFMT/master/SFMT-params2281.h)
Received 1.9KB (200 OK)

● Fetch(https://raw.githubusercontent.com/MersenneTwister-Lab/SFMT/master/SFMT-params4253.h)
Received 1.9KB (200 OK)

● Fetch(https://raw.githubusercontent.com/MersenneTwister-Lab/SFMT/master/SFMT-params11213.h)
Received 2KB (200 OK)

● Fetch(https://raw.githubusercontent.com/MersenneTwister-Lab/SFMT/master/SFMT-params44497.h)
Received 2KB (200 OK)

● Fetch(https://raw.githubusercontent.com/MersenneTwister-Lab/SFMT/master/SFMT-params86243.h)
Received 1.9KB (200 OK)

● Fetch(https://raw.githubusercontent.com/MersenneTwister-Lab/SFMT/master/SFMT-params132049.h)
Received 1.9KB (200 OK)

● Fetch(https://raw.githubusercontent.com/MersenneTwister-Lab/SFMT/master/SFMT-params216091.h)
Received 2KB (200 OK)

  Searched for 1 pattern, read 1 file, ran 1 shell command

● Background command "Run the crate's vector and scalar benchmark examples" completed (exit code 0)

  Searched for 2 patterns, ran 2 shell commands

● Background command "Re-run vector benchmark with target-cpu=native" completed (exit code 0)

  Made 2 scratchpad edits +32 -1, searched for 1 pattern, read 1 file, ran 10 shell commands, wrote 3 memories

● Fetch(https://raw.githubusercontent.com/Mersenne19937.out.txt)
Received 22KB (200 OK)

urng 1.0.0 コードレビュー

対象はこのリポジトリの作業コピーです。crates.io の urng-1.0.0 と内容が同一であること（改行コードの違いを除く）を確認しました。

A. 総評

設計の骨格は良くできています。Word/Rng トレイト  録している生成器の種類も幅広いです。MT19937、SFMT、Philox2x64/4x64、Threefry2x32、Threefish-256 の本体、xoshiro/xoroshiro の主要な変種、SFC32・JSF32 などは、参照実装や公式テストベクタと照合して一致しました（D節）。

一方で、1.0.0 として公開するには重大な問題が多く残っています。

- ビルド: Linux など大文字小文字を区別するファイルシステムではコンパイルできません。docs.rs のビルドも失敗しています。
- シード: ほぼ全生成器でシード 2k と 2k+1 が同じ乱数列になります。
- アルゴリズム: 名前どおりの実装になっていない生成器が複数あります（Sfc64、Xoroshiro128Pp、ChaCha/Salsa、Philox32 など）。
- SIMD と C ABI: safe API からのメモリ破壊、「AVX2」と表記しながら AVX-512VL を要求する型、一括生成での大量重複があります。

既存のテストは「同じシードなら同じ出力になるか」しか見ていません。そのため、この環境（AVX-512 搭載機）では単体テスト 101件・doctest 104件がすべて通り、上記の欠陥を1件も検出できませんでした。

B. 発見した問題

B-1. 確認済みの欠陥

以下はすべて、実行またはソース・アセンブリで裏付けています。

---

B1【Critical】大文字小文字を区別するファイルシステムでビルドできない

- 場所: src/lib.rs:1（include_str!("../Readme.md")）
- 問題: 実際のファイル名は README.md です。include_str! は通常のビルドでも評価されるため、Readme.md が見つからずコンパイルエラーになります。
- 発生条件・影響:
  - Linux（多くの CI を含む）、大文字小文字を区別する macOS ボリューム、docs.rs で発生します。
  - 依存クレートとして追加した時点で cargo build が失敗します。
  - 現在 docs.rs にドキュメントが存在しません。
- 根拠:
  - docs.rs のビルド #4683312（x86_64-unknown-linux-gnu）は couldn't read `src/../Readme.md`: No such file or directory で失敗しています。
  - 手元でも、fsutil file setCaseSensitiveInfo で大文字小文字を区別するディレクトリを作り、公開パッケージを置いて cargo check すると同じエラーで失敗しました。
- 改善案: パスを ../README.md に直し、Linux の C
- 確信度: 高

---

B2【Critical】Xoshiro256Ssx2: safe API なのにスタック外へ書き込み、出力も0に収束する

- 場所: src/prng/b64/xoshiro.rs:126-173
- 問題:
  a. safe 関数の nextu() が、_mm512_stream_si512 で 64バイトを [u64; 2]（16バイト）に書き込んでいます（170行目）。範囲外への書き込みに加え、64バイト境界が必要な NT ストアを、整列が保証されない配列に対して実行しています。
  b. 157行目は恒等シャッフル（0b11_10_01_00）との XOR、つまり s ^= s で、状態がゼロになります。
  c. new() も safe 関数なのに、AVX-512F 命令を無条件に実行します。
  d. 型名は ** ですが、doc とコメントは ++ と書いており、実際の計算はどちらでもありません。
- 発生条件・影響: Xoshiro256Ssx2::new(s).nextu() を呼ぶだけで発生します。
- 根拠（実行）:
  - #![forbid(unsafe_code)] を付けたクレートからそのまま呼び出せることを確認しました（safe API であることの確認）。
  - debug ビルドでは、この呼び出しで STATUS_ACCESS_VIOLATION（0xC0000005）が発生し、3回中3回クラッシュしました。
  - 最適化ビルドでは今回はクラッシュしませんでしたが、24回目以降の出力がすべて [0, 0] になりました。
  - 同じ intrinsic の列を再現したコードでも、32ステップで全レーンがゼロになりました。
- 改善案:
  - 書き直すまでは公開 API から外します（C ABI 側は別実装で、この関数を使っていません）。
  - 書き直す際は、状態を SoA（4本の __m512i）で持ち、ストアは [u64; 8] への storeu にし、new/nextu を unsafe かつ #[target_feature] 付きにします。
- 確信度: 高

  - 同じ intrinsic の列を再現したコードでも、32ステップで全レーンがゼロになりました。
- 改善案:
  - 書き直すまでは公開 API から外します（C ABI  いません）。
  - 書き直す際は、状態を SoA（4本の __m512i）で持ち、ストアは [u64; 8] への storeu にし、new/nextu を unsafe かつ #[target_feature] 付きにします。
- 確信度: 高

---

B3【Critical】C ABI の一括生成で出力の大半が重複

- 場所:
  - src/cabi/prng/b64/sfc.rs:160-170（sfc64x8_next_u64s）
  - biski.rs:157-167 ほか（biski64x8_*）
  - cet.rs:269-278（cet64x8_next_u64s）
  - xoshiro.rs:166-175（xoshiro256ssx2_next_u64s）
  - src/cabi/prng/b32/xoroshiro.rs:42（xoroshiro64ss_*）
- 問題:
  - (a) 64bit SIMD の4系統: レーンのシードを chunk_base + (group*8+i)*STRIDE（chunk_base = seed + chunk_idx*STRIDE）で作っています。このためストリームの番号が chunk_idx + lane になり、隣り合うチャンク同士で1本を除く全ストリームを共有します。
  - (b) xoroshiro64ss_*: チャンクシードが seed + chunk_idx の連番です。B4 の seed | 1 と組み合わさり、隣り合う2チャンクがまったく同じ内容になります。
- 発生条件・影響: (a) は関数ごとのチャンク長（4096〜131072要素）を超える count で、(b) は 2×131072要素を超える count で発生します。bench.md が測っている規模（1億要素）がまさにこれに当たります。シミュレーション用途では致命的な相関になります。
- 根拠（実行）:

| 関数                     | 要素数   | 異なる値の割合 |
|--------------------------|----------|----------------|
| sfc64x8_next_u64s        | 2^20     | 0.0248         |
| biski64x8_next_u64s      | 2^20     | 0.0362         |
| cet64x8_next_u64s        | 2^20     | 0.0771         |
| xoshiro256ssx2_next_u64s | 4×131072 | 0.2734         |
| 対照：sfc64_next_u64s    | 2^20     | 1.0000         |
  - 実測値は「異なるストリーム数 ＝ チャンク数＋レーン数−1」というモデルと一致します（例：sfc64x8 は 303/12288 ≈ 0.0247）。
  - sfc64x8 では buf[4096+j-1] == buf[j]（j=1..47）が成立しました。
  - xoroshiro64ss_next_u32s は、シード 1・2・3 のいずれでも、チャンク1とチャンク2が完全に一致しました。
- 改善案: レーンごとのシードを重ならない添字（chunk_idx*LANES + lane）を混合して作ります。理想的には jump/advance で逐次ストリームを分割します（pcg32x8 がすでにこの方式です）。重複がないことを確認するテストも追加します。
- 確信度: 高

---

B4【High】seed | 1 により、全生成器でシード 2k と 2k+1 が同じ乱数列になる

- 場所:
  - src/prng/b32/splitmix.rs:30、src/prng/b64/splitmix.rs:23
  - 同じ処理が wide/splitmix.rs:33、wide/pcg.rs:33、wide/squares.rs:39、prng/b64/xoroshiro.rs:23 にもあります。
- 問題: SplitMix で奇数である必要があるのは加算定数（gamma）だけで、状態は任意の値でかまいません。それなのに最下位ビットを潰しています。ほぼすべての生成器が new() の中で SplitMix を経由するため、影響が全体に及びます。さらに TwistedGFSR::new(u64) は seed as u32 で上位32ビットを捨てています。
- 発生条件・影響:
  - スレッド番号や試行番号を連番シードにする典型的な使い方で、2本ずつ同じ乱数列になります。実効的なシード空間も半分になります。
  - C ABI 内部の再シード処理にも波及します。
- 根拠（実行）: 次の27型すべてで、2000組中2000組の先頭32出力が一致しました。
  - SplitMix32、Sfc32、Jsf32、Pcg32、Mt19937、Sfmt19937、Xoshiro128Pp、Xoroshiro64Ss、Xorshift32、Xorwow、Philox32、Threefry32x2、Threefry32x4、Squares32、ChaCha20
  - SplitMix64、Xoshiro256Pp、Xoshiro256Ss、Xoroshiro128Ss、Sfc64、Biski64、Cet64、Mt1993764、Philox64、Threefish256、WyHash64、TwistedGFSR
  - あわせて、TwistedGFSR::new(5) == new(5 + 2^3
  - SplitMix32、Sfc32、Jsf32、Pcg32、Mt19937、Sfmt19937、Xoshiro128Pp、Xoroshiro64Ss、Xorshift32、Xorwow、Philox32、Threefry32x
    2、Threefry32x4、Squares32、ChaCha20
  - SplitMix64、Xoshiro256Pp、Xoshiro256Ss、Xoroshiro128Ss、Sfc64、Biski64、Cet64、Mt1993764、Philox64、Threefish256、WyHash64、TwistedGFSR
  - あわせて、TwistedGFSR::new(5) == new(5 + 2^32) も成立しました。
- 改善案: | 1 を削除し、64bit シードは全ビットを使います。これは出力が変わる変更（value-breaking）なので、バージョンを上げて
  CHANGELOG に明記します。1.0.0 は Linux でビル われるので、直すなら今が好機です。
- 確信度: 高

---

B5【High】shuffle/shuffled が Fisher–Yates ではなく、偏った並びを返す

- 場所: src/shuffle.rs:51-54, 70-73
- 問題: 入れ替え先 j を [i, len) ではなく [0, len) から選んでいます（いわゆるナイーブシャッフル）。
- 発生条件・影響: 要素数3以上で常に偏ります。
- 根拠（実行）: n=3 で600万回試行しました。一様なら各並びが 0.1667 になるはずですが、3つの並びが 0.222、残り3つが 0.111 になりました（理論値 2/9 と 1/9 に一致）。
- 改善案: for i in (1..len).rev() { let j = rng.to_index(i + 1); src.swap(i, j) } に置き換え、並びの分布を検定するテストを追加します。
- 確信度: 高

---

B6【High】TwistedGFSR が Rng の契約を満たさず、choice/shuffle が壊れる

- 場所: src/prng/b64/twisted_gfsr.rs:56-70
- 問題: 32bit の生成器なのに Word = u64 として 2^32 未満の値を返しています。randi などは上書きで補正していますが、Word::to_index は上書きできず、補正が効きません。
- 発生条件・影響: choice は常に添字0を返し、shuffle はシードに関係なく同じ並びになります。
- 根拠（実行）: 100シード×1000回の choice がすべて添字0でした。200シードで shuffle の結果は1種類だけでした。
- 改善案: Word = u32 に変更し、README の64bitの表からも外します。
- 確信度: 高

---

B7【High】safe API から未定義動作に到達できる（健全性の問題）

- 場所と問題:
  - (a) CPU 機能を確認しない safe 関数があります。
    - Xorshift32x8::new/nextuv（prng/b32/xorshif
    - Xoshiro256Ssx2（B2）
    - Sfc64x8、Biski64x8、Cet64x8、Cet256x2 の new
    - WordV の __m256i/__m512i 実装（rngv.rs:61-115）
  - (b) src/cabi/** の pub extern "C" fn が、safe 関数のまま生ポインタを参照外ししています。#![allow(clippy::not_unsafe_ptr_arg_deref)]       でリントを抑止したうえで、クレートのルートにD 版の *_new は CPU機能を確認しません（機能を確認する dispatch_simd! は定義されていますが、未使用です）。
  - (c) count == 0 で早期 return しない関数が多く、(NULL, 0) を渡すと slice::from_raw_parts_mut の前提に違反します（例：cabi/prng/b32/mersen
  - (d) NT ストアの後に _mm_sfence を呼んでいません。_stream_* は169か所あるのに対し、sfence は _internal.rs:293
    の1か所だけです。
- 発生条件・影響:
  - (a) は該当機能のない CPU で不正命令によりクラッシュします。
  - (b) は NULL や解放済みのポインタ（二重解放など）を渡すと、その時点で UB になります。
  - (d) は stdarch のドキュメントが UB と定めている条件に当たります。
- 根拠（実行・読解）:
  - #![forbid(unsafe_code)] のクレートで、次の呼び出しがコンパイル・実行できることを確認しました：bytemuck::cast で作った __m256i に対する WordV::to_randi（AVX-512VL を使用）、Xorshift32x8、Xoshiro256Ssx2、mt19937_new/mt19937_free。
  - debug ビルドで mt19937_next_f32s(h, NULL, 0) を呼ぶと、「unsafe precondition(s) violated: slice::from_raw_parts_mut requires the pointer to be aligned and non-null」で abort しました。
  - (d) の根拠は、_mm512_stream_si512 と _mm_sfence の doc にある「Safety of non-temporal stores」の節です。
- 改善案:
  - C ABI を pub unsafe extern "C" fn にし、ルートへの再エクスポートをやめます。
  - NULL と count == 0 のガードを全関数で統一します。
  - SIMD 版の _new は機能を確認し、非対応なら NULL を返します。
  - SIMD 型は、unsafe かつ #[target_feature] 付きの構築関数を唯一の入口にし、メソッドにも同じ機能集合を指定します。
  - NT ストアのループの後に _mm_sfence() を入れます。
- 確信度: 高（(d) が実際に問題を起こすかは B-2 で扱います）

---

B8【High】「AVX2」と表記した型が AVX-512F/VL の命令を使っている

- 場所:
  - prng/b32/sfc.rs:210, 227（Sfc32x8）
  - jsf.rs:108-109（Jsf32x8）
  - xoroshiro.rs:119, 122（Xoroshiro64Ssx8）
  - xorshift.rs:127（Xorshift32x8）
  - rngv.rs:77
  - C ABI の sfc32x8_*、xoroshiro64ssx8_*
- 問題: _mm256_rol_epi32、_mm256_mask_blend_epi32、_mm256_loadu_epi32 は avx512f,avx512vl を要求する intrinsic です（rust-src の stdarch で確認）。一方、Safety の記述と README は avx2 しか要求していません。
- 発生条件・影響: 利用者が AVX2 を正しく確認してから使っても、AVX-512 を持たない CPU（Zen 1〜3、Alder Lake 以降の Intel コンシューマ向けなど）ではクラッシュします。
- 根拠: release ビルドのアセンブリで、sfc32x8_nepi32 を11回 call しており、その本体は vprold$21,(%rdx),%ymm0（AVX-512VL の命令）でした。
- 改善案:
  - AVX2 で実装する場合は、回転を (x<<r)|(x>>(32-r)) に、ブレンドを _mm256_blend_epi32 に置き換えます。
  - AVX-512VL を前提にする場合は、要求機能の記述と README を修正します。
- 確信度: 高（AVX2 専用機での実行はしていません）

---

B9【High】ChaCha8/20・Salsa8/20 は暗号ストリームではなく、1ブロック見れば以降を完全に予測できる

- 場所: crng/b32/chacha.rs:25-71、salsa.rs:25-60、crng/mod.rs:3-4
- 問題:
  - 定数・鍵・nonce・カウンタの配置がなく、16語すべてを SplitMix32 の出力で埋めています。
  - feed-forward（最後に入力を足し戻す処理）がありません。
  - ROUNDS を「ダブルラウンド数」として使っているため、ChaCha20 は実際には40ラウンドになっています。
  - 出力したブロックがそのまま次の内部状態になります。
  - Salsa の3番目の回転量が 9 になっています（仕様は 13、salsa.rs:40）。
  - シードは u32 です（実効 2^31、B4 による）。
- 発生条件・影響: 名前から CSPRNG を期待した利用者にとって危険です。doc も「key/counter state」と、実際の実装とは異なる説明をしています。
- 根拠（実行）:
  - 自前の参照実装は RFC 8439 §2.3.2 の期待値 e4e7f110… を再現しました。同じ状態を urng
    に注入した出力は、「20ダブルラウンド・feed-f致しました。
  - ChaCha20::new(2024) の2番目のブロックは、1番目のブロックから完全に予測できました。
  - Salsa の出力は回転量 9 の場合と一致し、13 の
- 改善案: RFC 8439 に準拠して再実装するか rand_chacha に委譲します。それができない場合は、名前を変えて「暗号用途には使えない」と明記します。
- 確信度: 高

---

B10【High】名前どおりでない実装による品質・周期の劣化

- 発生条件・影響: シードに関係なく、該当する型を使えば必ず発生します。

┌─────────────────────────────┬───────────────────────────────┬───────────────────────────────────────────────────────────┐
│            対象             │             問題              │                       根拠（実行）                        │
├─────────────────────────────┼───────────────────────────────┼───────────────────────────────────────────────────────────┤
│ Sfc64（prng/b64/sfc.rs:47） │ c = rotl(c,24)+tmp            │ 参照の SFC64                                              │
│ 、Sfc64x8、C ABI 版         │ とすべきところが c =          │ と3出力目から一致しない。1万出力のすべてで「次の出力が直  │
│                             │ rotl(tmp,24) になっている     │ 前2つの出力から完全に計算できる」関係が成り立った         │
├─────────────────────────────┼───────────────────────────────┼───────────────────────────────────────────────────────────┤
│                             │                               │ 状態 [1,2] で urng は 3、参照の xoroshiro128++ は         │
│ Xoroshiro128Pp（prng/b64/xo │ 出力は s0+s1（+ 方式）で、状  │ 393217。GF(2) 行列の位数を調べると 2^128−1                │
│ roshiro.rs:86-95）          │ 態遷移も壊れている            │ にならず、README の周期は成り立たない。jump()             │
│                             │                               │ も自身の遷移と整合しない                                  │
├─────────────────────────────┼───────────────────────────────┼───────────────────────────────────────────────────────────┤
│ Pcg32（pcg.rs:27-33）、Pcg3 │                               │ シード 0〜2^22 の 50.0% で増分が偶数。シード 1150024      │
│ 2x8、wide::Pcg32x*          │ 増分 inc を奇数にしていない   │ は周期 2^45（jump-ahead で厳密に確認。README の記載は     │
│                             │                               │ 2^64）                                                    │
├─────────────────────────────┼───────────────────────────────┼───────────────────────────────────────────────────────────┤
│ SplitMix32x16（AVX-512      │ 混合関数 fmix32  上位13ビットの差分が、100万回で5種類しか現  │
│ 版、splitmix.rs:120-130）と │ の乗算を加算に置き換えている  │ れない（スカラー版は8192種類）                            │
│  C ABI 版                   │                               │                                                           │
├─────────────────────────────┼───────────────────────────────┼───────────────────────────────────────────────────────────┤
│ wide::SplitMix32x{4,8,16}（ │ 各レーンが隣のレーンを1ステッ │ 1回目 [96a0f96b,12bc8390,971e9964,79adc7e7]、2回目        │
│ wide/splitmix.rs:31-63）    │ プずらしたものにe9964,79adc7e7,591c8dd8]                     │
└─────────────────────────────┴───────────────────────────────┴───────────────────────────────────────────────────────────┘

- 改善案: それぞれ参照実装どおりに直します。いずれも value-breaking な変更なので、B4 とまとめて出すのが効率的です。
- 確信度: 高

---

B11【High】Philox4x64 と Threefish256 の先頭出力が常に0になる

- 場所: cbrng/b64/philox.rs:129（pos: 2、正しくは 4）、threefish.rs:83（pos: 0）
- 問題: 内部バッファの読み出し位置の初期値が誤っており、未生成の（ゼロ初期化された）バッファから値を返します。
- 発生条件・影響: どのシードでも、Philox4x64 は最初の2出力が、Threefish256 は最初の4出力が0になります。
- 根拠（実行）: Philox4x64::new(123) は [0, 0, …]、Threefish256::new(123) は [0, 0, 0, 0, …] を返しました。なお、計算本体は公式テストベクタ（KAT）と一致しています（D節）。
- 改善案: 初期位置をバッファ長と同じ値にします。
- 確信度: 高

---

B12【Medium】名前・参照実装との非互換（統計への

- 発生条件・影響: 他の実装（Random123、NumPy、std::mt19937 など）と出力を突き合わせる用途で一致しません。Xoroshiro64Ssx* は下位ビットの品質も劣ります。

┌──────────────────────────────────────┬───────────────────────────────────┬──────────────────────────────────────────────┐
│                 対象                 │               内容                │                 根拠（実行）                 │
├──────────────────────────────────────┼───────────────────────────────────┼──────────────────────────────────────────────┤
│ Philox32（cbrng/b32/philox.rs:64-112 │ 20ラウンドで、鍵の更新が乗算（key │ 「20ラウンド・乗算」の模擬実装と一致。カウン │
│ ）                                   │  *= W）                           │ タ0・鍵0では出力が [0,0,0,0]                 │
│                                      │                                   │ になる（公式値は 6627e8d5…）                 │
├──────────────────────────────────────┼───────────────────────────────────┼──────────────────────────────────────────────┤
│                                      │ 2つの乗算が両方とも同じ定数 M0    │                                              │
│ Philox32x4x4（Rust 版・C ABI 版）    │ を使い 参照の Philox4x32-10 と一致しない            │
│                                      │ っていない                        │                                              │
├──────────────────────────────────────┼───────────────────────────────────┼──────────────────────────────────────────────┤
│ Xoroshiro64Ssx8/x16（xoroshiro.rs:11 │ 出力が xoroshiro64*               │                                              │
│ 5, 183）                             │ の方式（s0*0x9E3779BB）で、**     │ 1出力目からスカラー版と一致しない            │
│                                      │ ではない                          │                                              │
├──────────────────────────────────────┼───────────────────────────────────┼──────────────────────────────────────────────┤
│ Biski64x8（biski.rs:124-126、C ABI   │ loop_mi2出力目からスカラー版と一致しない            │
│ 版も同様）                           │ を更新後の値から計算している      │                                              │
├──────────────────────────────────────┼───────────────────────────────────┼──────────────────────────────────────────────┤
│ Xoroshiro128Ss::long_jump（xoroshiro │ ジャン 2^96 ステップ先の状態と一致しない（jump()    │
│ .rs:148-171、Pp 版も同じ）           │ 上書きしている                    │ は2^64 ステップ先と一致した）                │
├──────────────────────────────────────┼───────────────────────────────────┼──────────────────────────────────────────────┤
│ Threefry32x4                         │ Random1カウンタ=1                                   │
│                                      │ と、出力へのカウンタの XOR がある │ のとき最下位ビットだけが公式値と異なる       │
├──────────────────────────────────────┼───────────────────────────────────┼──────────────────────────────────────────────┤
│ Mt19937 などの new                   │ 参照実装のシード方式 init_genrand │ Mt19937::new(5489) =                         │
│                                      │  を使っていない                   │ 2295094276（std::mt19937 は 3499211612）     │
└──────────────────────────────────────┴───────────────────────────────────┴──────────────────────────────────────────────┘

- 改善案: 参照どおりに直すか、doc に「参照実装と互換でない」ことと理由を明記し、互換のコンストラクタを別に用意します。
- 確信度: 高

---

B13【Medium】範囲指定の乱数生成で契約違反、SIMD 版にも誤り

- 場所: rng.rs:56-65、rngv.rs:36-52、cabi/** の rand_i32s
- 問題・根拠（実行）:
  a. randi の as iN + min が、範囲の幅が 2^31 を超え、かつ min<0 のとき debug ビルドでオーバーフローします。randi(i32::MIN, i32::MAX) で panic しました。debug ビルドの C ABI 経由では abort になります。
  b. min > max の場合の挙動が決まっていません。
  c. 拒否ステップのない multiply-shift 方式なので、doc の「uniformly」に反します。範囲 3·2^30 で添字を3で割った余りが0になる確率が 0.5001（一様なら 0.333）でした。
  d. randf(100, 101) が上限の 101.0 を返しました（2000万回中46回）。f64 でも同様です。半開区間 [min, max) の約束に反します。
  e. WordV for __m128i::to_randi の結果が [9, fffffff6, 9, fffffff6] でした（正しくは [9, 9, 9, 9]）。
  f. SIMD 版の rand_i32s(MIN, MAX) は、全要素が i32::MIN になりました（jsf32x16、sfc32x4、philox32x4x4 で確認）。
- 改善案:
  - 加算を wrapping_add にし、Lemire 法に拒否ステップを加えます。
  - randf の結果が上限と等しくなった場合に補正します。
  - 範囲 2^32 を特別扱いします。
  - __m128i 版は、正しく実装されている Sfc32x4::randiv の方式に統一します。
- 確信度: 高

---

B14【Medium】C ABI 一括生成の意味論（再シード・

- 場所: _internal.rs:353-420 を使う関数群、cabi/cbrng/b32/philox.rs:922-973
- 問題:
  - 多くの関数は、呼び出しのたびにハンドルから1語だけ取り出し、32bit シードで生成器を作り直しています。
  - その結果、出力が逐次ストリームと一致しません。
  - 生成器ごとに意味論がばらばらです。逐次生成すmp/カウンタで正しく分割するもの（pcg32x8、threefry など）が混在しています。
  - 小さな要求を繰り返すと、誕生日衝突で同じブロックが再び現れます。
  - philox32x4x4 は出力がバッファのアドレスに依存し、非整列のときカウンタの進め方が1ブロック不足します。
- 発生条件・影響: 呼び出し回数が多いほど、また出力の再現性を前提にするほど影響が大きくなります。
- 根拠（実行）:
  - mt19937_next_u32s の出力は、Mt19937 を逐次呼び出した結果と一致しませんでした。
  - 4語の要求を30万回呼ぶと、同じブロックが xoshiro128pp で25回、mt19937 で51回出現しました。
  - philox32x4x4 は、同じシードでもバッファを4バイトずらすと添字15から出力が分岐しました。さらに、ある呼び出しの最後の値が次の呼び出しの最初の値と重複しました。
- 改善案: pcg32x8 の pcg32_advance_lcg と同じ方式（逐次ストリームを jump/advance で分割）に統一します。
- 確信度: 高

---

B15【Medium】Bst32/64・Alias* の境界条件

- 場所: sampler/b32/bst.rs:48-57、b64/bst.rs:48-57、alias.rs:83-88
- 問題:
  - binary_search_by は値が完全一致したときその位置を返すため、乱数が累積値の境界とちょうど等しいと、重み0の要素が選ばれることがあります。
  - 空の重みを与えると、debug では panic、release では添字0を返します。Alias は空の重みで panic します。
- 発生条件・影響: 重み0の要素が、まれに選ばれます。
- 根拠（実行）:
  - 固定値を返す RNG で境界ちょうどの乱数を作ると、重み0の添字1が返りました。
  - 通常の RNG で5000万回引いたところ、重み0の要素が3回選ばれました。
- 改善案: partition_point(|&c| c <= r) を使い、
- 確信度: 高

---

B16【Medium】simd/full feature が x86_64 以外でコンパイルできない

- 場所: rngv.rs:1, 6 などの、条件なしの use std::arch::x86_64::*
- 問題: アーキテクチャ判定なしで x86_64 専用モジュールを使っています。
- 発生条件・影響: Apple Silicon などの aarch64  るとビルドできません。
- 根拠（実行）: aarch64-apple-darwin 向けのクロスチェックで、既定 feature は成功し、--features simd は unresolved import `std::arch::x86_64`  で失敗しました。
- 改善案: cfg(target_arch = "x86_64") で統一的にガードします。
- 確信度: 高

---

B17【Medium】rand 連携の不備

- 場所: _internal.rs:633-636, 669-671、rand/b64/mod.rs
- 問題・根拠（実行）:
  - 64bit 生成器には TryRng/Rng が実装されていません（next_u64 を呼ぶとコンパイルエラーになることを確認）。
  - シード型が [u8; 4] で、Xoshiro256Pp なども32bitのシードしか受け取れません。
  - from_ne_bytes を使っているため、CPU のバイト順で結果が変わります（rand_core の再現性ガイドラインに反します）。
  - 32bit 生成器の next_u64 は1語目を上位に置いており、fill_bytes のリトルエンディアン順と矛盾します（実測で確認）。
  - Jsf32、Threefry、ChaCha などが実装対象から漏れています。
- 発生条件・影響: rand エコシステムで 64bit 生成器が使えず、シード空間も狭くなり、プラットフォーム間で出力が一致しない可能性があります。
- 改善案: シードを各生成器の状態幅に合ったバイト列にして from_le_bytes で読み、rand_core::utils の補助関数を使います。
- 確信度: 高

---

B18【Medium】テストの構造上、上記の欠陥を検出できない

- 場所: _internal.rs:528-570、各モジュールの cfg(target_feature) によるゲート、prng/b32/sfc.rs:348-350
- 問題・根拠（実行）:
  - テストは決定性の確認だけで、公式テストベクタの照合も分布の検定もありません。
  - SIMD のテストはコンパイル時の機能判定に隠れており、通常のビルドでは1件もコンパイルされません。-C target-cpu=native で有効にすると、36件のコンパイルエラーになりました。
  - Sfc32x8/x16 のテストは実行時の CPU 判定なしで AVX-512 命令を実行します。
  - 置き場所を誤った #[cfg(feature = "simd")] のせいで、スカラーの test_sfc32 が既定の feature では実行されません。
- 発生条件・影響: B2〜B15 の欠陥がいずれもテストを素通りしています。
- 改善案: C節の3を参照してください。
- 確信度: 高

---

B19【Medium】ベンチマークの再現性と公平性

- 場所: bench.md、examples/cabi/test.rs
- 問題:
  - DRAM モードでは、ほぼすべての生成器がメモリ帯域の上限（約27GB/s）に張り付いており、生成器の速度差を測れていません。
  - ビルドフラグとスレッド数が書かれていません。
  - 3回中の最大値を報告しています。
  - 出力の正しさを確認しないまま計測しています（B3 の重複出力のままの数字です）。
- 発生条件・影響: bench.md の数字を、ビルド設定を知らない第三者が再現できません。性能の主張の根拠にもなりにくい状態です。
- 根拠（実行、同じ 9800X3D で計測）:

| 項目                         | 既定フラグ | target-cpu=native | bench.md   |
|------------------------------|------------|-------------------|------------|
| xoshiro128ppx16（L3 モード） | 0.82 GS/s  | 40.02 GS/s        | 46.21 GS/s |
| philox32                     | 3.67 GS/s  | 6.31 GS/s         | 3.55 GS/s  |
  - アセンブリを見ると、RngV::nextuv がインライン化されず、その中の intrinsic 14個がすべて関数呼び出しになっていました。RngV
    の実装に #[target_feature] がないためです。
  - bench.md の値はどちらの設定とも一貫せず、.cargo/ がバージョン管理から除外されているので、計測時の設定も再現できません。
- 改善案:
  - ビルドフラグ・スレッド数・中央値を明記します。
  - 単一スレッドで計算速度を測るベンチマークを別に用意します。
  - 計測の前に出力を検証します。
  - RngV の実装に #[target_feature] を付けます。
- 確信度: 高（計測は各1回です）

---

B20【Low】ドキュメントと実装の不一致

- README の wide のサンプル use urng::wide::{WRng, …} はコンパイルできません（実際の名前は RngW。実行して確認）。
- mt19937_new(uint32_t seed, size_t warm) と書か はありません。
- 表の Philox32x4 は、実際には Philox32 です。
- Sfc32x4 は simd feature が必要です。
- TwistedGFSR が 64bit の表に載っています。
- Sfc64x8/Biski64x8 の doc に、事実と異なる「4-way」「AVX2」の記述があります。
- Xorshift128 の doc にある「OR-ed with 1」は実
- rustdoc の警告が9件あります。
- cfg_attr(not(doctest)) のため、README のコードは一度も検証されていません。
- 確信度: 高

B21【Low】全出力がゼロになるシードがある

- Xorshift32 には全出力が0になるシードが6個あります（0x61C88646 など。u32 の全シードを総当たりで確認）。Xorshift64 にも 0x61C8864680B583EA/…EB の2個があります。
- rand_core は、このようなシードを別の値に写し替えることを推奨しています。
- 確信度: 高

B22【Low】API と保守性の細かい点（好みを含む）

- RNG 型が Copy を実装しています。rand_core は、うっかり状態が複製されるのを防ぐため Copy を実装しないよう推奨しています。
- align(64) により、16バイトの状態を持つ型も64バイトになります。
- 空スライスに対して shuffle は Err を返し、choice は panic します。
- Sampler が &mut R を保持し続けます。
- thiserror が未使用の依存として残っています。
- build.rs が通常のビルドのたびにソースツリー（README）へ書き込みます。

B-2. 追加検証が必要な懸念（未確定）

1. 作者独自・改変版の生成器（Cet64/Cet256 の小さな定数（−189、−229）と s[2] を使わない点、Biski64x8 の改変漸化式、Squares の鍵の選び方）の統計品質。PractRand/TestU01 での確認はしていません。
2. AVX2 しかない CPU で実際にクラッシュするかどうか。stdarch の属性とアセンブリからの推論で、実機では試していません。
3. sfence がないことの実害。x86 では rayon 内部 たれている可能性があります。
4. B2 のスタック外書き込みが、release ビルドで何を壊しているか。
5. ビッグエンディアン環境での SFMT の lshift128（bytemuck::cast::<u128>）の挙動。
6. Xoroshiro128Pp の実際の周期構造（最大周期でないことまでしか示していません）。

C. 改善提案（優先度順）

1. 1.0.1 を即時リリース: B1 を修正し、Linux CI と docs.rs のビルドを通します。
2. 重大欠陥の撤去または修正: B2・B3 の該当 API を、直すまで非公開にします。yank も検討してください。
3. テスト基盤の整備:
   - 公式テストベクタとの照合テスト（本レビューで使った値：MT 3499211612、MT64 14514284786278117030、SFMT 3440181298…、Random123・Skein・RFC 8439）
   - SIMD とスカラーのレーン一致テスト（既存の sfc32x4_lanes_match_scalar を全 SIMD 型に広げる）
   - 範囲と分布のプロパティテスト
   - CI のマトリクス：既定フラグ、target-cpu=native、Intel SDE -hsw（AVX2 のみの CPU を模擬）、Linux、aarch64
4. 出力が変わる修正をまとめて出す: B4・B10〜B12 を一度にまとめ、メジャーバージョン（または明示した minor）として CHANGELOG に列挙します。
5. SIMD の安全設計を徹底する: 「CPU 機能の確認を型で証明する」パターンを全体に適用します（unsafe + #[target_feature]
   の構築関数、メソッドにも同じ機能集合、C ABI  B19）を同時に改善できます。
6. C ABI: unsafe extern "C" 化、NULL ガード、sfence の追加、名前空間の分離、逐次ストリームと互換な意味論への統一を行います。
7. Rng API: 拒否ステップ付きの Lemire 法、randf  明文化、choice の戻り値を Option にします。
8. 性能:
   - Philox32 を正規の10ラウンドにします（現在はスカラーで 0.26 GS/s。半分のラウンド数なので約2倍速くなる見込みですが、未計測です）。
   - Cet256x2 の撤去を検討します。C ABI の計測で です（bench.md：2.37 対 2.93、私の再計測：2.84 対 3.19 GS/s）。
   - BISKI64_PAR_CHUNK = 4 は、4要素ごとに再シードしているので見直します。
9. ドキュメント: 参照実装との違い、暗号用途に使えないこと、型ごとに必要な CPU 機能、ベンチマークの条件を明記します。

D. 評価できる設計

- 参照実装と一致することを確認した実装:
  - MT19937、MT19937-64
  - SFMT19937（公式の出力ファイルの先頭10値と一致）と、他の9変種のパラメータ（公式ヘッダと全項目一致）
  - Philox2x64-10・Philox4x64-10・Threefry2x32-20（Random123 の公式テストベクタと一致）
  - Threefish-256 の本体（Skein 1.3 の公式テストベクタと一致）
  - SFC32、JSF32、xoshiro128/256 の ++ と 、xoroshiro64、xoroshiro128**、Xorwow、Xorshift128、WyHash64、PCG の出力関数（各10万出力で一致）
  - Xoroshiro128Ss::jump()（2^64 ステップ先と一致）
  - SIMD の Sfc32x16（全レーンが参照の SFC32 と一致）、Cet64x8（スカラー版と一致）
- Word/Rng の sealed トレイト設計: 実装者は next 範囲への変換が1か所にまとまっています。
- pcg32x8 の C ABI: LCG の jump-aheadでチャンクの開始状態を計算しており、スレッド数に関係なく同じ出力になります。threefry、squares、スカラー版 philox もカウンタ方式で正しく分割しています。B3・B14 の修正にそのまま流用できる手本です。
- fill_chunk_nt: 小さすぎるバッチでは通常のストアにフォールバックし（回帰テスト付き）、sfence も呼んでいます。NT
  ストアを使うかどうかをバッファ全体の大きさで判
- #[target_feature] 付きの safe な構築関数（Sfc32x8::new など）: Rust の新しい仕組みを使った正しい方向の設計です。
- simd_f01: スカラー版と SIMD 版で浮動小数への変換結果をビット単位で揃える方針と、それを確かめる同値テストがあります。
- SeedGen: RDSEED/RDRAND を実行時に検出し、失敗時のリトライも備えています。

E. 検証状況と限界

- 同一性の確認: ローカルの作業コピーと crates.io の 1.0.0 が同一であることを確認しました。すべての実行は、スクラッチ領域に複製した公開パッケージ（build.rs を含まない）で行い、リポジトリは変更していません（git status は開始時と同じ）。リポジトリ内にある target/debug は私の操作で作られたものではありません（作成元は確認していません）。
- 読んだ範囲:
  - src/ の非 cabi ファイルは全部、examples/、README、Cargo.toml、build.rs、bench.md。
  - cabi は、mersenne・jsf・splitmix・xorshift・ 、その他の主要部分を部分的に読みました。b64 のcet/mersenne/splitmix/twisted_gfsr/xorshift/xoroshiro と、b64 の cbrng（philox/threefish）は grep で構造を確認しただけです。
  - 依存クレート：wrapn、stdarch、rand_core 0.10.1、csbindgen、bytemuck。urng_core は
    Cargo.toml・Cargo.lock・ソースのどこにも出て
- 実行したもの（Windows 11、Ryzen 7 9800X3D（AVX-512 F/VL/DQ）、rustc 1.98.1）:
  - 単体テスト：既定 feature 48件、full 101件、すべて通過。
  - doctest：104件すべて通過。
  - 大文字小文字を区別するディレクトリでのビルド確認、docs.rs のログ確認、aarch64 向けクロスチェック、target-cpu=native でのテストのコンパイル。
  - 自作の検証クレート：参照実装、状態の注入、GF(2) 行列計算、統計の集計、C ABI のテスト、forbid(unsafe_code) での到達性確認。
  - 付属ベンチマーク：既定フラグと target-cpu=native の2条件。
  - アセンブリの確認。
- 実施していないもの:
  - AVX2 のみ・AVX なしの CPU での実行
  - 実機の Linux
  - ビッグエンディアン環境
  - PractRand/TestU01
  - Miri
  - C# バインディングと cribler の確認
  - ベンチマークの繰り返し計測（各1回）
- 検証用クレートの場所: スクラッチ領域の verify と verify_compile に残しています。

このレビューを共有用の非公開ページ（Artifact）として公開することもできます。