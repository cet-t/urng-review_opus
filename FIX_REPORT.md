# urng 1.0.0 修正レポート（作業途中：利用枠の上限により中断）

基準コミット: `4068051`（phase 1: review。crates.io の urng 1.0.0 と同一のソース）。
レビュー指摘番号（B1〜B22）は `REVIEW.md` と対応しています。

注: 共同著作者トレーラー追加のため履歴を書き換えました（内容は不変、ツリーID一致を確認）。
本ファイルのハッシュは書き換え後のものです。旧履歴はタグ `pre-coauthor-4c26943` に残っています。

## 進捗（作業途中で中断した場合はここから再開）

| # | 指摘 | 状態 | コミット |
|---|---|---|---|
| 1 | B1 Readme.md のパス | 完了 | 80f367e |
| 2 | B2 Xoshiro256Ssx2 | 完了 | 04aa92d |
| 3 | B3 C ABI 一括生成の重複 | 完了 | 22bb053 |
| 4 | B4 seed \| 1 | 完了 | 3673130 |
| 5 | B5 shuffle | 完了 | c6ff383 |
| 6 | B6 TwistedGFSR | 完了 | a96aa53 |
| 7 | B11 先頭出力ゼロ | 完了 | f26c0c4 |
| 8 | B10 アルゴリズム欠陥 | 完了 | bc8ec6a |
| 9 | B12 Biski64x8 | 完了 | 8abacf2 |
| 10 | B9 ChaCha/Salsa | 完了 | c8cfb16 |
| 11 | B8 AVX2型のAVX-512VL使用 | 完了 | adb9ae0 |
| 12 | B7 健全性 | 完了 | adb9ae0 |
| 13 | B12 残り（Xoroshiro64Ssx8/x16、Philox32、Philox32x4x4、Threefry32x4、MT/SFMT シード） | 完了 | adb9ae0, 78740a1 |
| 14 | B13 範囲生成（a〜f） | 完了 | adb9ae0, 21d946e |
| 15 | B15 サンプラ | 未着手 | |
| 16 | B16 非x86 | 未着手 | |
| 17 | B17 rand連携 | 未着手 | |
| 18 | B14 C ABI意味論 | カウンタ方式（philox32/x4x4、threefry32x4/x2、philox64、threefish256）は完了。**未完了**: 逐次 PRNG の一括生成（チャンクごとの再シード）の意味論 | 78740a1 |
| 19 | B18 テスト基盤 / B20 docs / B21 | 一部（Sfc32 テストの cfg 修正のみ） | |
| 20 | 全体検証・修正後ベンチマーク | 未着手 | |

## 各修正の内容とテスト

| 指摘 | 修正 | 追加テスト（すべて通過） |
|---|---|---|
| B1 | `include_str!("../README.md")`。README を doctest 対象にし、壊れていた wide のサンプル（`WRng`）を `urng::wide` の doctest に移動 | `tests/packaging.rs`（ファイル名の大文字小文字一致。基準版で失敗を確認） |
| B2 | `Xoshiro256Ssx2` を SSE2 のみの正しい xoshiro256** ×2 に書き直し（safe API のまま健全）。C ABI は逐次生成 | 公開ベクタ（xoshiro256**/++）、レーン=スカラー、ゼロ収束なし、C ABI=逐次 `nextu`、重複なし |
| B3 | 64bit SIMD 一括生成のレーン番号を `chunk*LANES+lane` に。xoroshiro64ss のチャンクシードを `chunk_seed32` に | 2^20 出力で重複なし、チャンク不一致（基準版で失敗を確認） |
| B4 | `seed \| 1` を全廃（SplitMix32/64、wide、Xoroshiro128Pp）。TwistedGFSR は64bitシードを全ビット使用 | `tests/seeding.rs`（全スカラー・wide型で隣接シードが異なる、SplitMix64=参照） |
| B5 | Fisher–Yates（Durstenfeld）に変更 | n=3,4 の置換分布のカイ二乗検定 |
| B6 | `TwistedGFSR::nextu` を TT800 出力2つで64bit化、上書き実装を削除 | TT800 参照比較、`choice` 一様性、シード依存の `shuffle` |
| B11 | `Philox4x64`/`Threefish256` の初期 `pos` をバッファ長に | Random123 KAT、Skein 1.3 KAT（tweak 付き参照比較）、先頭ブロック一致 |
| B10 | Sfc64 漸化式、Xoroshiro128++ 本物化と long_jump 修正、PCG 増分を奇数化、SIMD SplitMix32 の fmix32 と重ならないレーン配置 | PractRand ベクタ（SFC32/SFC64）、pcg32-demo ベクタ、GF(2) 行列による最大周期と jump/long_jump の厳密検証、全レーン比較、C ABI カーネルの要素単位比較 |
| B12（Biski） | `loop_mix` を旧値から計算（Rust・C ABI） | 全レーン=スカラー |
| B9 | ChaCha を RFC 8439 準拠、Salsa20 を仕様準拠（回転13、feed-forward、正しいラウンド数）。`from_key` 追加、Debug で鍵を表示しない | RFC 8439 §2.3.2、eSTREAM Salsa20 Set1 #0、仕様の quarterround、参照比較、自己フィードバックしないこと |
| B7 | `RngV` を `unsafe trait`、`WordV` を `unsafe fn` に。各 SIMD 型に `#[target_feature]` の `step()`、機能を要する `new` も `#[target_feature]`。ハンドルを受ける C ABI 関数を `unsafe extern "C"` に、NULL ハンドル・NULL バッファ・`count==0` を許容（`from_raw_parts_mut(NULL,0)` の UB を解消）。SIMD の `_new` は `is_x86_feature_detected!` で非対応なら NULL。NT ストアを行う全関数の末尾に `_mm_sfence`。simd モジュールを `target_arch = "x86_64"` に限定。一度も実行されていなかった `unsafe_test!`（コンパイル時 cfg 依存、型検査すら通らない状態）を実行時検出のレーン比較に置換（Sfc32、Cet64x8/Cet256x2、Squares32x8） | `tests/source_invariants.rs`（NT フェンス、VL 命令不使用、C ABI の unsafe 化。基準版で 3 件とも失敗、違反 254 件）、`cabi_bulk::null_and_empty_buffers_are_accepted`（基準版は UB 事前条件違反で異常終了）、各レーン=スカラー比較、Cet64 ミキサの独立式比較 |
| B8 | `Jsf32x8`/`Sfc32x8`/`Xoroshiro64Ssx8`/`Xorshift32x8` を AVX2 のみで実装（`_mm256_rol_epi32`・`mask_blend`・`loadu_epi32` を排除） | `no_avx512vl_256bit_intrinsics`、各レーン=スカラー、JSF/xoroshiro64** の参照実装比較 |
| B12（xoroshiro64） | `Xoroshiro64Ssx8/x16` に ** スクランブラ `rotl(s0*0x9E3779BB,5)*5` | 参照実装比較、全レーン=スカラー |
| B13 | `Word::to_randi` は `min>max` で panic、全範囲でオーバーフローしない（`wrapping_add`）。`to_randf` は `max` を返さない。`to_index` は u128 で計算。`__m128i::to_randi` のレーン1・3の誤りを修正。SIMD の `rand_i32s` は `scale32`（0 = 2^32）で全範囲が定数にならない。64bit SIMD の `randi`/`randf` は `Word` 経由 | `rng::tests`（全範囲、逆転範囲、`max` 未満の網羅確認、巨大 len）、`to_randi_matches_scalar_for_all_widths`、`cabi_bulk::simd_full_range_rand_i32s_is_not_constant`（基準版で失敗）。基準版で `randi(i32::MIN,i32::MAX)` のデバッグ時オーバーフローと `randf(100,101)==101` を実行して確認 |
| B12（残り） | Philox32 を 10 ラウンド・加算の鍵更新に。Philox32x4x4（Rust・C ABI）は x2 に M1 を掛け、鍵の更新を k0/k1 のみに、10 ラウンドに。Threefry32x4 を Random123 Threefry4x32-20 に（tweak と出力へのカウンタ XOR を削除）。`Mt19937::from_init_genrand`・`Mt1993764::from_init_genrand64`・`Sfmt19937::from_init_gen_rand` を追加（参照互換。`new` の値は不変で、SplitMix 前処理であることを doc に明記） | Random123 KAT（philox4x32_10、threefry4x32_20、threefry2x32_20 各3件）、Philox32x4x4 の全レーン=参照（64bit 繰り上がりを含む）、C++ 標準が定める 10000 番目の出力（mt19937: 4123659995、mt19937_64: 9981545732273789042）、SFMT.19937.out.txt の先頭値 3440181298、独立実装の初期状態との一致 |
| B14（カウンタ方式） | `_internal::par_fill_ctr` を新設。philox32・threefry32x4/x2・philox64・threefish256 の一括生成は、逐次 `nextu` と完全に同じ列を返す（バッファ残りから開始、末尾を捨てない、128bit 繰り上がり、アライメント非依存）。philox32x4x4 の先頭非整列時の重複・ずれを解消（NT ストアは `fill_chunk_auto` が整列時のみ使用）。`rand_i*` で min > max は何も書かない（FFI 越しの panic は abort になるため） | `cabi_bulk::philox32_bulk_equals_sequential_stream`、`philox32x4x4_bulk_equals_sequential_blocks`、`counter_mode_bulk_equals_sequential_stream`（基準版で全件失敗を確認。x4x4 は最初の非整列バッファで失敗）、128bit 繰り上がりの単体テスト |
| B13（残り） | `Rng::randi` に Lemire 法の拒否ステップ（`Word::to_randi_uniform`／`to_index_uniform` を追加。sealed トレイトなので外部実装への影響なし）。shuffle と choice も正確な添字生成に。受理された値は従来と同一で、拒否されたまれな場合だけ列が変わる。C ABI の `rand_f32s/f64s` を `[min, max)` に収める（SIMD・手書きの 13 カーネルが max に丸まりえた）。C ABI の `rand_i32s/i64s` は min > max で何も書かず（42 関数すべて）、残っていた `as iN + min` を wrapping に、squares32x8 の全範囲が min に潰れる不具合を修正。ベクトル API（`RngV::randfv`、`WordV::to_randf`、`WRng::randi`）は1レーン1語で拒否できないため、丸め・偏りの上限を doc に明記 | `rng::tests`（範囲 3·2^30 の一様性、拒否の台本テスト、添字）、`cabi_bulk::c_abi_float_ranges_exclude_max`（基準版で 43/43 関数が失敗）、`cabi_bulk::c_abi_integer_ranges`（基準版はオーバーフローで異常終了） |

最終確認時点（21d946e）: `cargo test --features full` で 単体 152、結合 10+1+5+3（cabi_bulk/packaging/seeding/source_invariants）、doctest 112（README の ignore 2 件）すべて通過。`--no-default-features` と rand/cabi/wide/simd/sampler/seedgen/cabi+simd/simd+wide/full の各構成でテストのビルドがエラーなし（simd 単独の警告 13 件は基準版と同じ未使用定数）。

## 互換性への影響（ここまで）

- 出力が変わる（value-breaking）: 偶数シードの全生成器（B4）、TwistedGFSR、shuffle、Sfc64/Sfc64x8、Xoroshiro128Pp とそのジャンプ、Xoroshiro128Ss::long_jump、Pcg32 系、SplitMix32x16（AVX-512）と wide::SplitMix32xN（とそれで種まきされる wide::Jsf32xN）、Biski64x8、ChaCha/Salsa、各 C ABI 一括生成。メジャーバージョンでの公開を推奨。
- API: `ChaCha<R>`/`Salsa<R>` は `Copy` でなくなった。B7 により、ハンドルを受ける C ABI 関数は Rust からは `unsafe` 呼び出しが必要（C からの ABI・シグネチャは不変）。SIMD の `_new` は非対応 CPU で NULL を返す（従来は SIGILL）。`RngV` は `unsafe trait`、`WordV` のメソッドは `unsafe fn`、`Sfc64x8`/`Biski64x8`/`Cet64x8`/`Cet256x2`/`Xorshift32x8` の `new` は `#[target_feature]`（安全でない文脈からは `unsafe`）。`randi` は `min > max` で panic（従来は無意味な値）。`Xoshiro256Ssx2` の内部表現変更（公開 API は不変）。
- B13: `Rng::randi`・shuffle・choice は拒否ステップにより、まれに消費する語数が変わる（受理値は同一）。`Word` に必須メソッド 2 つを追加（sealed のため互換性問題なし）。C ABI の `rand_i*` は min > max で何も書かない（従来は無意味な値、または debug で abort）。
- 追加 API: `Mt19937::from_init_genrand`、`Mt1993764::from_init_genrand64`、`Sfmt19937::from_init_gen_rand`、C ABI `threefry32x2_rand_f32s`（他の型にはあり、これだけ欠けていた）。`Threefry32x4` は内部の `tw` フィールドを削除（非公開フィールド）。カウンタ方式の C ABI は、逐次 API と同じ列を返すよう出力が変わった（value-breaking）。Philox32/Philox32x4x4/Threefry32x4 も出力が変わった。
- 固定値テストの更新: `choice::tests::it_works` の期待値 3→7（B4 による意図した変化。テスト自体は維持）。

## 性能

- 修正前ベンチマークは基準 worktree で取得済み（スクラッチ領域 `base_vector.txt` / `base_scalar.txt`）。**修正後の計測は未実施**。
- 予想される低下: `xoshiro256ssx2_next_u64s` は並列 AVX-512 から逐次 SSE2 に変更（正しさ優先。重複のない並列化には xoshiro256 の jump 実装が必要）。ChaCha20 は 40→20 ラウンドで高速化、Salsa は同等の見込み（未計測）。

## 残存リスク

上表の未完了・未着手項目（B14 の逐次 PRNG 一括生成、B15〜B17、B20/B21）は未修正のまま残っています。C ABI の命名不整合を確認: `sfmt1993764` の浮動小数範囲関数が `sfmt_rand_f64s` という名前で公開されている（未修正）。AVX2 のみ・AVX-512 非搭載の実機では未検証（本機は AVX-512 搭載。AVX2 型は静的検査とコード上の命令で確認）。
