# NameSpec と Names

穴に入る名前は、名前空間ごとに用意した**候補**の中から選ばれます。候補の作り方を決めるのが `NameSpec` です。

## 正規表現で候補を作る

`space(名前空間, 正規表現, 個数)` で、正規表現に合う互いに異なる名前を、指定した個数だけ作ります。

```rust
fn names() -> NameSpec {
    NameSpec::new()
        .space(VAR, "[a-c]", 3)
        .space(FUNC, "[a-c]", 3)
}
```

名前空間を複数使うときは `.space` を続けて書きます。

候補は proptest が作ります。そのため、失敗した入力を縮小すると、名前も正規表現の範囲で短く単純なものになります。

## Strategy を渡す

正規表現で書けない候補は、`space_with` で Strategy を直接渡します。予約語と重なる名前を避けたいときに使用します。

```rust
const KEYWORDS: [&str; 5] = ["let", "print", "del", "fn", "call"];

fn names() -> NameSpec {
    let var = prop::collection::hash_set(
        "[a-z]{1,4}".prop_filter("keyword", |s| !KEYWORDS.contains(&s.as_str())),
        3,
    )
    .prop_map(|set| set.into_iter().collect::<Vec<_>>());
    NameSpec::new().space_with(VAR, var.boxed())
}
```

## 固定の候補

候補を固定したいときは、`space_with` に proptest の `Just` を渡します。特定の名前で起きるバグを再現したいときや、単体テストに向いています。

```rust
let names = NameSpec::new().space_with(VAR, Just(vec!["x".to_string(), "y".to_string()]).boxed());
```

## 候補の数の決め方

候補の数は生成される入力の性質を大きく変えます。

**少ないとき**は同じ名前が何度も使われます。再定義やシャドーイング、同じ名前の削除と再定義が自然に起こり、名前解決器にとって難しい入力になります。

**多いとき**は名前がばらけて関係が単純になります。名前どうしの衝突で起きるバグは見つかりにくくなります。

一方で、少なすぎると制約を満たせなくなります。

- `distinct_defs`(再定義なし)は、定義の数が候補の数を超えると満たせない
- 「見えていない名前を参照する」は、その時点ですべての候補が見えていると満たせない

満たせない入力は棄却されるので、棄却が多いときは候補の数を見直してください。
