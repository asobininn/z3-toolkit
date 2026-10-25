# Space と穴

穴は「後で名前が入る空欄」です。どの穴にどの名前が入るかは、制約をもとにZ3が決めます。この節では、穴を作るための名前空間(`Space`)と、穴が持つ情報を説明します。

## 名前空間を作る

名前空間は互いに独立した名前の集まりです。変数と関数のように、別々に解決される名前ごとに1つ作ります。

```rust
const VAR: Space = Space::new("var");
const FUNC: Space = Space::new("func");
```

`Space::new()` には `&'static str` を渡します。この文字列は、反例の表示などにそのまま使用されます。

名前空間が違えば、同じ名前の文字列でも関係を持ちません。

## 穴の役割

穴は名前空間と**役割**(`Role`)を持ちます。役割は次の3つです。

| 役割 | 作り方 | 意味 |
| --- | --- | --- |
| `Role::Def` | `VAR.def()` | 名前を定義する |
| `Role::Use` | `VAR.refer()` | 名前を参照する |
| `Role::Kill` | `VAR.kill()` | 名前を削除する。以降、その名前は見えなくなる |

`Use`と`Kill`の穴をまとめて**参照**と呼びます。どちらも既にある名前を指す穴だからです。制約の中で参照をまとめて扱う方法は、[Binds と規則](./binds.md)で説明します。

`def()` / `refer()` / `kill()`は、穴を1つだけ含む `Frag` の Strategy を返します。役割を引数で選びたいときは `hole(role)` を使用します。

```rust
VAR.hole(Role::Kill)   // VAR.kill() と同じ
```

## 例: 削除と関数を追加する

Getting Started の言語に、`del x` と、関数の `fn f` / `call f` を追加します。

```rust
fn stmt() -> impl Strategy<Value = Frag> {
    prop_oneof![
        VAR.def().prop_map(|x| frag!["let ", x]),
        (VAR.def(), VAR.refer()).prop_map(|(x, y)| frag!["let ", x, " = ", y]),
        VAR.refer().prop_map(|y| frag!["print ", y]),
        VAR.kill().prop_map(|x| frag!["del ", x]),
        FUNC.def().prop_map(|f| frag!["fn ", f]),
        FUNC.refer().prop_map(|f| frag!["call ", f]),
    ]
}
```

テスト対象の名前解決器も、`del` と関数を扱えるように広げてください。

> **削除後の参照を作りたいとき**: 「削除した名前を使う」入力は、`del` の後に同じ名前空間の参照が来る構造からしか作れません。構造は proptest が決めるので、`del` の出現率が低い Strategy では、制約を満たせずに棄却が増えます。このような場合は、`del` を出やすくするか、`del` の後に参照を置く文を Strategy の側で用意してください。

## ヒント

各穴は、名前とは別に**ヒント**を持ちます。ヒントは「候補の何番目の名前が欲しいか」を表す0～1の固定小数点数(`Hint(u16)`)で、proptest が決めます。

Z3 は制約とぶつからない限りヒントどおりの名前を選びます。そのため、制約で決まらない部分の名前は proptest によってばらつきます。

ヒントは縮小にも使用されます。縮小するとヒントは 0 に寄るので、穴に入る名前の候補も先頭に寄っていきます。反例で同じ名前が繰り返し使われやすいのはこのためです。

ヒントを利用者が直接扱う必要はありません。穴を`defs()`などで作れば、ヒントも一緒に作られます。
