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
| 11 | B8 AVX2型のAVX-512VL使用 | **未着手**（設計のみ。下記） | |
| 12 | B7 健全性 | **未着手**（設計のみ。下記） | |
| 13 | B12 残り（Philox32/Philox32x4x4/Xoroshiro64Ssx8・x16/Threefry32x4/MTシード） | 未着手 | |
| 14 | B13 範囲生成（randi/randf、`__m128i::to_randi`、SIMDの全範囲） | 未着手 | |
| 15 | B15 サンプラ | 未着手 | |
| 16 | B16 非x86 | 未着手 | |
| 17 | B17 rand連携 | 未着手 | |
| 18 | B14 C ABI意味論（再シード、philox32x4x4 のアライメント依存） | 未着手 | |
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

最終確認時点（c8cfb16）: `cargo test --features full` で 単体 129、結合 3+1+5、doctest 108（README の ignore 2 件）すべて通過。

## 互換性への影響（ここまで）

- 出力が変わる（value-breaking）: 偶数シードの全生成器（B4）、TwistedGFSR、shuffle、Sfc64/Sfc64x8、Xoroshiro128Pp とそのジャンプ、Xoroshiro128Ss::long_jump、Pcg32 系、SplitMix32x16（AVX-512）と wide::SplitMix32xN（とそれで種まきされる wide::Jsf32xN）、Biski64x8、ChaCha/Salsa、各 C ABI 一括生成。メジャーバージョンでの公開を推奨。
- API: `ChaCha<R>`/`Salsa<R>` は `Copy` でなくなった。`Xoshiro256Ssx2` の内部表現変更（公開 API は不変）。
- 固定値テストの更新: `choice::tests::it_works` の期待値 3→7（B4 による意図した変化。テスト自体は維持）。

## 性能

- 修正前ベンチマークは基準 worktree で取得済み（スクラッチ領域 `base_vector.txt` / `base_scalar.txt`）。**修正後の計測は未実施**。
- 予想される低下: `xoshiro256ssx2_next_u64s` は並列 AVX-512 から逐次 SSE2 に変更（正しさ優先。重複のない並列化には xoshiro256 の jump 実装が必要）。ChaCha20 は 40→20 ラウンドで高速化、Salsa は同等の見込み（未計測）。

## 未完了の設計メモ（B7/B8 の次の手順）

1. `RngV` を `unsafe trait` にし、`WordV` のメソッドを `unsafe fn` にする（`__m256i` は bytemuck で safe に作れるため）。`to_randi` は AVX2 版を `_mm256_blend_epi32` で、`__m128i` 版はレーン1・3のバグを直し、scale=0 を 2^32 とみなす。C ABI の `v_range` は `set1_epi32(scale32(min,max))` に統一する。
2. `Jsf32x8`/`Sfc32x8`/`Xoroshiro64Ssx8` の `_mm256_rol_epi32` を AVX2 のシフト＋OR に置き換える（B8）。`Xoroshiro64Ssx8/16` に ** スクランブラを追加する（B12）。
3. `Xorshift32x8::new`、`Sfc64x8`/`Biski64x8`/`Cet64x8`/`Cet256x2::new` を `#[target_feature]` 付きにする。C ABI は `unsafe extern "C"` 化、NULL/`count==0` ガード、SIMD の `_new` は機能非対応なら NULL を返す、NT ストア後に `_mm_sfence()`。
4. examples/cabi/test.rs は C ABI の unsafe 化に合わせて更新が必要。

## 残存リスク

上表の未着手項目（特に B7 健全性、B8 の AVX2 機での SIGILL、B13 の範囲生成、B14 の C ABI 再シード重複）はすべて未修正のまま残っています。
