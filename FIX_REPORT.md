# urng 1.0.0 修正レポート（最終）

基準コミット: `4068051`（phase 1: review。crates.io の urng 1.0.0 と同一のソース）。
レビュー指摘番号（B1〜B22）は `REVIEW.md` と対応しています。

注: 共同著作者トレーラー追加のため履歴を書き換えました（内容は不変、ツリーID一致を確認）。
本ファイルのハッシュは書き換え後のものです。旧履歴はタグ `pre-coauthor-4c26943` に残っています。
push は行っていません。

## 進捗

| # | 指摘 | 状態 | コミット |
|---|---|---|---|
| 1 | B1 Readme.md のパス | 完了 | 80f367e |
| 2 | B2 Xoshiro256Ssx2 | 完了 | 04aa92d |
| 3 | B3 C ABI 一括生成の重複 | 完了 | 22bb053 |
| 4 | B4 seed \| 1 | 完了 | 3673130 |
| 5 | B5 shuffle | 完了 | c6ff383 |
| 6 | B6 TwistedGFSR | 完了 | a96aa53, 89ba463（性能） |
| 7 | B11 先頭出力ゼロ | 完了 | f26c0c4 |
| 8 | B10 アルゴリズム欠陥 | 完了 | bc8ec6a |
| 9 | B12 Biski64x8 | 完了 | 8abacf2 |
| 10 | B9 ChaCha/Salsa | 完了 | c8cfb16 |
| 11 | B8 AVX2型のAVX-512VL使用 | 完了 | adb9ae0 |
| 12 | B7 健全性 | 完了 | adb9ae0 |
| 13 | B12 残り（Xoroshiro64Ssx8/x16、Philox32、Philox32x4x4、Threefry32x4、MT/SFMT シード） | 完了 | adb9ae0, 78740a1 |
| 14 | B13 範囲生成（a〜f） | 完了 | adb9ae0, 21d946e |
| 15 | B14 C ABI 意味論 | 完了 | 78740a1, 0e4a3c1, 89ba463（性能） |
| 16 | B15 サンプラ | 完了 | af76385 |
| 17 | B16 非 x86 | 完了（修正は B7 の cfg 統一、検証は本表下） | adb9ae0 |
| 18 | B17 rand 連携 | 完了 | 58c4b60 |
| 19 | B21 全ゼロ出力のシード | 完了 | b97b8d4 |
| 20 | B18 テスト基盤 | 完了 | adb9ae0, 18d849a ほか各修正 |
| 21 | B20 ドキュメント | 完了 | 18d849a |
| 22 | B19 ベンチマーク | 一部（RngV の target_feature 化と計測条件の明記。bench.md は未更新） | adb9ae0 |
| 23 | B22 API・保守性 | 未修正（下記「未修正の項目」） | |
| 24 | 全体検証・修正後ベンチマーク | 完了 | — |

## 各修正の内容とテスト

| 指摘 | 修正 | 追加テスト（すべて通過） |
|---|---|---|
| B1 | `include_str!("../README.md")`。README を doctest 対象にし、壊れていた wide のサンプル（`WRng`）を `urng::wide` の doctest に移動 | `tests/packaging.rs`（ファイル名の大文字小文字一致。基準版で失敗を確認） |
| B2 | `Xoshiro256Ssx2` を SSE2 のみの正しい xoshiro256** ×2 に書き直し（safe API のまま健全）。C ABI は逐次生成 | 公開ベクタ（xoshiro256**/++）、レーン=スカラー、ゼロ収束なし、C ABI=逐次 `nextu`、重複なし |
| B3 | 64bit SIMD 一括生成のレーン番号を `chunk*LANES+lane` に。xoroshiro64ss のチャンクシードを分離 | 2^20 出力で重複なし、チャンク不一致（基準版で失敗を確認） |
| B4 | `seed \| 1` を全廃（SplitMix32/64、wide、Xoroshiro128Pp）。TwistedGFSR は64bitシードを全ビット使用 | `tests/seeding.rs`（全スカラー・wide型で隣接シードが異なる、SplitMix64=参照） |
| B5 | Fisher–Yates（Durstenfeld）に変更 | n=3,4 の置換分布のカイ二乗検定 |
| B6 | `TwistedGFSR::nextu` を TT800 出力2つで64bit化、上書き実装を削除。性能低下を受けて2語を1回の境界判定で読む高速経路を追加（出力不変） | TT800 参照比較、`choice` 一様性、シード依存の `shuffle`、`nextu_is_two_raw_outputs`（twist をまたぐ組を含む） |
| B11 | `Philox4x64`/`Threefish256` の初期 `pos` をバッファ長に | Random123 KAT、Skein 1.3 KAT（tweak 付き参照比較）、先頭ブロック一致 |
| B10 | Sfc64 漸化式、Xoroshiro128++ 本物化と long_jump 修正、PCG 増分を奇数化、SIMD SplitMix32 の fmix32 と重ならないレーン配置 | PractRand ベクタ（SFC32/SFC64）、pcg32-demo ベクタ、GF(2) 行列による最大周期と jump/long_jump の厳密検証、全レーン比較、C ABI カーネルの要素単位比較 |
| B12（Biski） | `loop_mix` を旧値から計算（Rust・C ABI） | 全レーン=スカラー |
| B9 | ChaCha を RFC 8439 準拠、Salsa20 を仕様準拠（回転13、feed-forward、正しいラウンド数）。`from_key` 追加、Debug で鍵を表示しない | RFC 8439 §2.3.2、eSTREAM Salsa20 Set1 #0、仕様の quarterround、参照比較、自己フィードバックしないこと |
| B7 | `RngV` を `unsafe trait`、`WordV` を `unsafe fn` に。各 SIMD 型に `#[target_feature]` の `step()`、機能を要する `new` も `#[target_feature]`。ハンドルを受ける C ABI 関数を `unsafe extern "C"` に、NULL ハンドル・NULL バッファ・`count==0` を許容（`from_raw_parts_mut(NULL,0)` の UB を解消）。SIMD の `_new` は `is_x86_feature_detected!` で非対応なら NULL。NT ストアを行う全関数の末尾に `_mm_sfence`。simd モジュールを `target_arch = "x86_64"` に限定。一度も実行されていなかった `unsafe_test!`（コンパイル時 cfg 依存、型検査すら通らない状態）を実行時検出のレーン比較に置換し、マクロを削除 | `tests/source_invariants.rs`（NT フェンス、VL 命令不使用、C ABI の unsafe 化。基準版で 3 件とも失敗、違反 254 件）、`cabi_bulk::null_and_empty_buffers_are_accepted`（基準版は UB 事前条件違反で異常終了）、各レーン=スカラー比較、Cet64 ミキサの独立式比較 |
| B8 | `Jsf32x8`/`Sfc32x8`/`Xoroshiro64Ssx8`/`Xorshift32x8` を AVX2 のみで実装（`_mm256_rol_epi32`・`mask_blend`・`loadu_epi32` を排除） | `no_avx512vl_256bit_intrinsics`、各レーン=スカラー、JSF/xoroshiro64** の参照実装比較 |
| B12（xoroshiro64） | `Xoroshiro64Ssx8/x16` に ** スクランブラ `rotl(s0*0x9E3779BB,5)*5` | 参照実装比較、全レーン=スカラー |
| B13 | `Word::to_randi` は `min>max` で panic、全範囲でオーバーフローしない（`wrapping_add`）。`to_randf` は `max` を返さない。`to_index` は u128 で計算。`__m128i::to_randi` のレーン1・3の誤りを修正。SIMD の `rand_i32s` は `scale32`（0 = 2^32）で全範囲が定数にならない。64bit SIMD の `randi`/`randf` は `Word` 経由 | `rng::tests`（全範囲、逆転範囲、`max` 未満の網羅確認、巨大 len）、`to_randi_matches_scalar_for_all_widths`、`cabi_bulk::simd_full_range_rand_i32s_is_not_constant`（基準版で失敗）。基準版で `randi(i32::MIN,i32::MAX)` のデバッグ時オーバーフローと `randf(100,101)==101` を実行して確認 |
| B12（残り） | Philox32 を 10 ラウンド・加算の鍵更新に。Philox32x4x4（Rust・C ABI）は x2 に M1 を掛け、鍵の更新を k0/k1 のみに、10 ラウンドに。Threefry32x4 を Random123 Threefry4x32-20 に（tweak と出力へのカウンタ XOR を削除）。`Mt19937::from_init_genrand`・`Mt1993764::from_init_genrand64`・`Sfmt19937::from_init_gen_rand` を追加（参照互換。`new` の値は不変で、SplitMix 前処理であることを doc に明記） | Random123 KAT（philox4x32_10、threefry4x32_20、threefry2x32_20 各3件）、Philox32x4x4 の全レーン=参照（64bit 繰り上がりを含む）、C++ 標準が定める 10000 番目の出力（mt19937: 4123659995、mt19937_64: 9981545732273789042）、SFMT.19937.out.txt の先頭値 3440181298、独立実装の初期状態との一致 |
| B14（カウンタ方式） | `_internal::par_fill_ctr` を新設。philox32・threefry32x4/x2・philox64・threefish256 の一括生成は、逐次 `nextu` と完全に同じ列を返す（バッファ残りから開始、末尾を捨てない、128bit 繰り上がり、アライメント非依存）。philox32x4x4 の先頭非整列時の重複・ずれを解消（NT ストアは `fill_chunk_auto` が整列時のみ使用）。`rand_i*` で min > max は何も書かない（FFI 越しの panic は abort になるため）。性能低下を受けて `CtrRing::blocks` によるバッチ計算を追加（threefry は下位 64bit が桁あふれしないバッチを高速経路で計算） | `cabi_bulk::philox32_bulk_equals_sequential_stream`、`philox32x4x4_bulk_equals_sequential_blocks`、`counter_mode_bulk_equals_sequential_stream`（基準版で全件失敗を確認。x4x4 は最初の非整列バッファで失敗）、128bit 繰り上がりの単体テスト、`batched_blocks_handle_the_64_bit_carry` |
| B13（残り） | `Rng::randi` に Lemire 法の拒否ステップ（`Word::to_randi_uniform`／`to_index_uniform` を追加。sealed トレイトなので外部実装への影響なし）。shuffle と choice も正確な添字生成に。受理された値は従来と同一で、拒否されたまれな場合だけ列が変わる。C ABI の `rand_f32s/f64s` を `[min, max)` に収める（SIMD・手書きの 13 カーネルが max に丸まりえた）。C ABI の `rand_i32s/i64s` は min > max で何も書かず（42 関数すべて）、残っていた `as iN + min` を wrapping に、squares32x8 の全範囲が min に潰れる不具合を修正。ベクトル API（`RngV::randfv`、`WordV::to_randf`、`WRng::randi`）は1レーン1語で拒否できないため、丸め・偏りの上限を doc に明記 | `rng::tests`（範囲 3·2^30 の一様性、拒否の台本テスト、添字）、`cabi_bulk::c_abi_float_ranges_exclude_max`（基準版で 43/43 関数が失敗）、`cabi_bulk::c_abi_integer_ranges`（基準版はオーバーフローで異常終了） |
| B14（逐次 PRNG） | 2 分類に統一し `cabi` モジュールの doc に明記。(1) 逐次ストリームと完全一致: カウンタ方式、jump-ahead を持つもの（`Pcg32::advance`・`SplitMix32::advance` を新設し `par_fill_advance` で並列化。pcg32・splitmix32 が対象に加わった）、逐次生成のもの。squares32x8 はブロック単位で進めるよう修正（count 単位で進めていたため、次の呼び出しでカウンタとレーンの鍵の対応がずれていた）。(2) jump-ahead のない生成器はサブストリーム方式のまま、呼び出しごとに 64bit の鍵を引き、各チャンクの生成器を状態全体にわたって初期化（32bit 型に crate 内 `from_seed64` を追加。`new(seed)` の値は `SeedWords` により不変） | `advanceable_bulk_equals_sequential_stream`（基準版で失敗）、`squares_bulk_equals_sequential_stream`（修正前のコードで失敗）、`repeated_small_calls_do_not_repeat_blocks`（4語×20万回。基準版は 7 種すべてで 7〜21 個の重複ブロック、修正後は 0。SIMD 5 種も 0）、`Pcg32::advance` の単体テスト |
| B15 | Bst32/64 を `partition_point(\|c\| c <= r)` に（重み0の要素を境界で選ばない）。全サンプラで重みを検証し、空・負・非有限・合計0 なら明示的なメッセージで panic（従来は debug でアンダーフロー、release で範囲外の添字0） | 定数 RNG による境界ちょうどのテスト（基準版は重み0の添字1を返すことを確認）、重み0が4万回で一度も選ばれないこと、空・全0・負の重みの拒否 |
| B16 | B7 で simd 関連のモジュール・import をすべて `target_arch = "x86_64"` でガード | `cargo +nightly check -Zbuild-std=core,std,alloc --target aarch64-apple-darwin --lib` を既定・simd・cabi・wide・full で実行し全てエラー0（基準版は simd で 37 件、full で 53 件のエラー）。aarch64 実機でのテスト実行はしていない |
| B17 | 32bit 型: シードを little-endian で読む（`from_ne_bytes` を廃止）、`next_u64` は1語目を下位に（rand_core 規約、`fill_bytes` と一致）。64bit 型: `TryRng` を実装（従来なし）、`Seed = [u8; 8]` で 64bit 全域、`next_u32` は上位半分、`fill_bytes` は LE。Jsf32・Threefry32x2/x4・Philox4x64・ChaCha8/20・Salsa8/20 を追加（暗号系は 256bit 鍵をシードに） | `rand32_conventions`（基準版で `next_u64` の語順が逆であることを実行で確認）、`rand64_conventions`、`cipher_seed_is_the_key` |
| B21 | SplitMix32 が全ゼロ状態に写すシード 0x61C88647・0xD7BDEF1B・0xF7AA06C3（全 2^32 を総当たりで特定）と、Xorshift64 のシード −γ を固定の非ゼロ状態に写す。他のシードの列は不変。Xorshift128 の doc の誤り（存在しない「OR 1」）を訂正 | `xorshift32_has_no_all_zero_seed`、`xorshift64_has_no_all_zero_seed`（基準版で4シードとも全出力0を確認） |
| B18 | 公式ベクタ・参照実装比較・分布検定・SIMD 全レーン比較・C ABI の要素単位比較を各修正で追加。コンパイル時 cfg 依存のテストを全廃（`unsafe_test!` 削除）、SIMD テストは実行時検出 | 上記各テスト。`source_invariants` に csbindgen 入力の検査を追加 |
| B20 | README の型表（Philox32、SSE2 型、Xorshift32x8・Philox4x64、ラウンド数、AVX-512DQ の要否）、SIMD 構築関数と CPU 判定の説明、C ABI の誤ったシグネチャ（`warm`）と規約を修正。`sfmt1993764_rand_f64s` を正式名とし `sfmt_rand_f64s` は非推奨の別名として残す。rustdoc 警告 0 | `tests/readme_types.rs`、README の doctest |
| B19 | `RngV` 実装の `#[target_feature]` 化（B7）で intrinsic がインライン化されるようになった（xoshiro128ppx16 L3: 0.84→40.9 GS/s）。計測条件は本書「性能」に明記 | — |

補足: 修正の途中で、`build.rs`（csbindgen がソースを字句解析して C# バインディングを生成）が `macro_rules!` 生成の関数を拾えず、threefry の C ABI 11 関数が C# から消えていたことに気づき、関数を明示的に書き戻した（18d849a）。再生成したバインディングは基準版の 212 エントリをすべて含み、追加 2 件（`threefry32x2_rand_f32s`、`sfmt1993764_rand_f64s`）。

## 最終検証（HEAD = 89ba463）

- `cargo test`（単体・結合・doctest）を全 feature 構成で実行し、すべて失敗 0:
  - 既定 149 / rand 154 / cabi 160 / wide 229 / simd 184 / sampler 170 / seedgen 151 / cabi+simd 200 / simd+wide 264 / full 308 件通過（doctest の ignore は README の2件のみ）。
- 警告: simd 単独と simd+wide の 9 件のみ。いずれも基準版にもある未使用定数・メソッド（基準版は 13 件）。それ以外の構成は 0。
- rustdoc（full）警告 0。release ビルド（full）とすべての example のビルドが成功。
- aarch64-apple-darwin（nightly、build-std、`--lib`）: 既定・simd・cabi・wide・full でエラー 0。dev-dependency の `alloca` がクロス C コンパイラを要するため、aarch64 向けのテストのビルドと実行はしていない。
- 実行環境: Windows 11、Ryzen 7 9800X3D（AVX-512 F/VL/DQ）、Rust 1.98 stable。AVX2 のみの CPU、AVX-512 非搭載 CPU では実行していない。

## 互換性への影響

- 出力が変わる（value-breaking）: 偶数シードの全生成器（B4）、TwistedGFSR、shuffle、Sfc64/Sfc64x8、Xoroshiro128Pp とそのジャンプ、Xoroshiro128Ss::long_jump、Pcg32 系、SplitMix32x16（AVX-512）と wide::SplitMix32xN（とそれで種まきされる wide::Jsf32xN）、Biski64x8、ChaCha/Salsa、Philox32/Philox32x4x4/Threefry32x4、Xoroshiro64Ssx8/x16、Xorshift32/64 の全ゼロシード、各 C ABI 一括生成。メジャーバージョンでの公開を推奨。
- API: `ChaCha<R>`/`Salsa<R>` は `Copy` でなくなった。ハンドルを受ける C ABI 関数は Rust からは `unsafe` 呼び出しが必要（C からの ABI・シグネチャは不変）。SIMD の `_new` は非対応 CPU で NULL を返す（従来は SIGILL）。`RngV` は `unsafe trait`、`WordV` のメソッドは `unsafe fn`、`Sfc64x8`/`Biski64x8`/`Cet64x8`/`Cet256x2`/`Xorshift32x8` の `new` は `#[target_feature]`。`randi` は `min > max` で panic（従来は無意味な値）。`Xoshiro256Ssx2` の内部表現変更（公開 API は不変）。
- B13: `Rng::randi`・shuffle・choice は拒否ステップにより、まれに消費する語数が変わる（受理値は同一）。`Word` に必須メソッド 2 つを追加（sealed のため互換性問題なし）。C ABI の `rand_i*` は min > max で何も書かない（従来は無意味な値、または debug で abort）。
- B14: pcg32・splitmix32・squares32x8 の C ABI 一括生成と、サブストリーム方式の 32bit 生成器（jsf32、sfc32、xoshiro128、xoroshiro64、mt19937、sfmt、各 SIMD 版）の C ABI 出力が変わる。サブストリーム方式の C ABI 出力は、Rust の逐次 `nextu` とは一致しない（仕様として doc に明記）。
- B15: サンプラは不正な重みで panic する（従来は debug で算術 panic、release で誤った添字）。
- B17: 64bit 型の `SeedableRng::Seed` が `[u8; 4]` から `[u8; 8]` に変わった（型の変更。`seed_from_u64` 経由の利用は影響なし）。32bit 型の `from_seed` はビッグエンディアン環境で結果が変わる。32bit 型の `next_u64` の語順が逆になった。
- 追加 API: `Mt19937::from_init_genrand`、`Mt1993764::from_init_genrand64`、`Sfmt19937::from_init_gen_rand`、`Pcg32::advance`、`SplitMix32::advance`、C ABI `threefry32x2_rand_f32s`・`sfmt1993764_rand_f64s`、rand 連携の対象型の追加。`Threefry32x4` は内部の `tw` フィールドを削除（非公開フィールド）。`sfmt_rand_f64s` は非推奨（残置）。
- 固定値テストの更新: `choice::tests::it_works` の期待値 3→7（B4 による意図した変化。テスト自体は維持）。テストの削除・無効化はしていない（削除したのは、一度もコンパイルされていなかった `unsafe_test!` とその呼び出しで、実行時検出の同等以上のテストに置き換えた）。

## 性能

計測条件: 基準版（4068051）と修正後（89ba463）をそれぞれ別の git worktree から、既定フラグ（RUSTFLAGS なし、`--release`、Cargo.toml の profile は lto=fat・codegen-units=1）でビルドし、同じ機械で交互に各 5 回実行した中央値。`vector-test`（C ABI、N=1e8、ベンチマーク内部で 3 回中の最大値、DRAM モードと L3 常駐モード）と `scalar-test`（Rust の `Rng`、N=1e8）。DRAM モードは NT ストアの書き込み上限（約 27 GB/s = 6.9 GS/s u32、3.4 GS/s u64）に張り付くものが多く、生成速度の差は L3 モードと scalar の方が表れる。

±5% を超えて変化したもの（GS/s、基準→修正後）:

| モード | 生成器 | 基準 | 修正後 | 比 | 理由 |
|---|---|---|---|---|---|
| DRAM | xoshiro256ssx2 | 3.46 | 0.64 | 0.18 | B2: 誤った出力を並列生成していたものを正しい逐次 SSE2 に。重複のない並列化には xoshiro256 の jump による分割が必要（未実装） |
| L3 | xoshiro256ssx2 | 59.85 | 0.64 | 0.01 | 同上 |
| L3 | xoroshiro128pp | 12.09 | 10.05 | 0.83 | B10: 本物の ++ スクランブラ（演算が増えた） |
| L3 | xoroshiro64ssx16 | 41.52 | 36.23 | 0.87 | B12: ** スクランブラ（乗算 2 回と回転） |
| scalar | TwistedGFSR | 1.10 | 0.77 | 0.70 | B6: u64 1 語に TT800 の出力 2 語が必要（従来は 1 語で契約違反）。高速経路の追加後の値（追加前は 0.37） |
| scalar | Sfmt216091 | 1.19 | 1.09 | 0.92 | 原因未特定（Rust 側の生成コードは変更していない。コード配置の影響の可能性） |
| DRAM | threefish256 | 0.10 | 0.09 | 0.90 | B14: 逐次 `nextu` 1 語ずつ（末尾の語を捨てない正しい意味論） |
| DRAM | philox32 | 3.72 | 5.70 | 1.53 | B12: 20→10 ラウンド |
| DRAM | xoshiro128ppx16 / ssx16 | 3.00 / 2.62 | 6.90 / 6.91 | 2.3 / 2.6 | B7/B19: `#[target_feature]` の step で intrinsic がインライン化 |
| L3 | xoshiro128ppx16 | 0.84 | 40.91 | 48.7 | 同上 |
| L3 | philox32x4x4 | 7.36 | 17.54 | 2.38 | B12/B14: 10 ラウンド、汎用バッチ化 |
| L3 | sfc32x8 | 17.62 | 45.46 | 2.58 | B7/B8: target_feature によるインライン化 |
| scalar | Philox32 | 0.26 | 0.61 | 2.35 | 20→10 ラウンド |
| scalar | ChaCha8/20、Salsa8/20 | 0.34/0.15、0.35/0.15 | 0.59/0.27、0.60/0.28 | 1.7〜1.9 | B9: 仕様どおりのラウンド数 |

上記以外（Sfmt4253/86243/132049 の +6〜10% を除く）はすべて ±5% 以内。修正途中に threefry32x4/x2 の C ABI が 0.62 倍に落ちたが、正しさとは無関係な汎用化の副作用だったため 89ba463 でバッチ化し、基準と同等（1.00〜1.04 倍）に戻した。生データはスクラッチ領域の `bench_*_r{1..5}.txt` と `bench_summary.csv`。

## 未修正の項目

- B22（好みを含む API・保守性）: RNG 型の `Copy` 実装（削除は破壊的変更）、`align(64)` による型サイズ、空スライスでの shuffle（Err）と choice（panic）の不一致、Sampler が `&mut R` を保持すること、未使用の optional 依存 `thiserror`（暗黙の公開 feature になっているため削除は破壊的）、`build.rs` がビルドのたびに README を書き換えること（`build.rs` はパッケージから除外されているので crates.io 利用者には影響しない）。
- B19: `bench.md` の数値は更新していない（本書の計測で置き換えるべき）。単一スレッドの計算速度ベンチマークは追加していない。
- C ABI の `*_rand_i32s` と SIMD のベクトル API は 1 要素 1 語の multiply-shift のままで、偏りは `range / 2^BITS` 以下（doc に明記）。正確に一様なのは Rust の `Rng::randi`・shuffle・choice。
- SFMT 各変種（Sfmt607 など）の C ABI はマクロ生成のため、C# バインディング（build.rs）に含まれない（基準版から同じ。テストに既知の例外として記載）。また build.rs の入力一覧に sfc・xoroshiro（32bit）・biski の C ABI ファイルが含まれていない（基準版から同じ）。
- REVIEW.md B-2 の「追加検証が必要な懸念」（独自生成器の統計品質の PractRand/TestU01 検証、AVX2 のみの実機での動作、ビッグエンディアン環境での SFMT、Xoroshiro128Pp の周期構造の詳細）は未検証。

## 残存リスク

- AVX2 のみ・AVX-512 非搭載の CPU では未実行。AVX2 型が AVX-512VL 命令を含まないことは静的検査（`source_invariants`）とソース上の命令で確認したのみ。
- aarch64 は `check` のみで、テストは実行していない。
- サブストリーム方式の C ABI は、呼び出しごとに 64bit の鍵を使うため、呼び出しをまたいだ同一ブロックの再出現確率は約 (チャンク生成数)^2 / 2^65 で、実用上は無視できるが 0 ではない。
- 多数の出力値が変わったため、既存の利用者が固定値・再現性に依存している場合はメジャーバージョンでの告知が必要。
- 64bit の `to_f01` は語の上位ビット 11〜62 を使う（最上位ビットは指数部に重なり無視される）。一様性は保たれるが、32bit 版とビットの使い方が異なる（今回は変更していない）。
