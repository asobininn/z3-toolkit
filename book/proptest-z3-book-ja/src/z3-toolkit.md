# z3-toolkit で制約を書く

proptest-z3 の制約は、z3-toolkit の constraint! マクロで書きます。この章では、proptest-z3 で使う部分だけを説明します。proptest_z3::prelude::* が z3-toolkit の prelude を再エクスポートしているので、依存を足す必要はありません。

## `constraint!` は `z3::ast::Bool` を1つ作る

`constraint!(..)` は、中に書いた条件からZ3の `Bool` を1つ作ります。`fill` に渡す規則は `Bool` を返す関数なので、規則の本体は基本 `constraint!(..)` を使用します。

```rust
program().fill(names(), |binds| constraint!(
    bound(binds, FUNC),
    !distinct_defs(binds, VAR),
))
```

## 条件の書き方

| 書き方 | 意味 |
| --- | --- |
| `all { p, q, .. }` | すべて成り立つ |
| `any { p, q, .. }` | どれかが成り立つ |
| `exactly(n) { .. }` | ちょうど n 個が成り立つ |
| `at_most(n) { .. }` / `at_least(n) { .. }` | n 個以下 / n 個以上が成り立つ |
| `!p`、`p && q`、`p \|\| q` | 否定、かつ、または |
| `{ p } implies { q }` | p ならば q |
| `distinct(xs)` | すべて異なる |

## `for` と `if` で展開する

`constraint!`内では `for` と `if` が使用でき、要素ごとの条件に展開されます。次の例は「`VAR`の参照のうち、見えていないものがちょうど1つ」です。

```rust
constraint!( exactly(1) { for i in binds.refs(VAR) {
    !visible(binds, i)
} })
```

`if` はRustの `bool` で条件を絞ります。

`constraint!` は z3 の `Bool` を返す式なので、長くなった制約は `&Binds` を受け取って `Bool` を返す関数に切り出せます。次の例は「`VAR`の名前は再定義がない」です。

```rust
fn no_redefinition(binds: &Binds) -> Bool {
    let defs: Vec<_> = binds.defs(VAR).collect();
    constraint!(
        all { for &p in &defs { for &q in &defs {
            if p < q {
                binds.name[p] != binds.name[q]
            }
        } } },
    )
}
```

この例では、`distinct` を使うと、より簡潔に書けます。

```rust
fn no_redefinition(binds: &Binds) -> Bool {
    constraint!(
        distinct(binds.defs(VAR).map(|i| &binds.name[i])),
    )
}
```

## もっと知りたい場合

constraint! の機能一覧や詳しい使い方は、[z3-toolkit のREADME](https://github.com/asobininn/z3-toolkit) を参照してください。

また、パズルを制約として記述し、解を求める例を [Examples](https://github.com/asobininn/z3-toolkit/blob/main/crates/z3-toolkit/examples.md) で紹介しています。
