# EBNF Notation

The notation used in [grammar.ebnf](./grammar.ebnf) is as follows:

## Precedence
The precedence from highest to lowest is as follows:

* Grouping - `(` and `)`
* Postfix - `+`, `*`, and `?`
* Joining - `foo bar`
* Union - `|`

## Whitespace
Whitespace in a grammar file is insignificant.

## Comments
Any `#` that is not part of a string literal begins a comment that spans until the end of the line. Comments do not influence the behavior of the grammar.

## Rule Definitions

A rule name must match the regular expression `[a-z_][a-z0-9_]*`. The entry point of a grammar is the first rule definition. A rule can be referenced by name in itself and other rules. A rule definition must be terminated with `;`.

```
program ::= expr+;
```

A rule can be given a list of parameters (matching `$[a-z0-9_]+`) that can be referenced by name. A rule parameter acts as a parenthesized expression.

```
list[$item, $sep] ::= $item ($sep $item)*;
# Equivalent of '(parameter | spread) ("," (parameter | spread))*'
parameters ::= list[parameter | spread, ","];
```

A parameter can be given a default value with `=`.

```
list[$item, $sep=","] ::= $item ($sep $item)*;
```

## Tokens

Tokens can be referenced by their TokenKind ([lexer.rs](../src/parser/lexer.rs)) enum name.

```
literal ::= StringLiteral;
```

## Literals

A literal that must be matched in a rule can be written as a JSON string.

```
call_expr ::= primary_expr "(" arguments ")";
```

## Joining

Multiple items can be joined by being placed adjacent to one another.

```
paren_expr ::= "(" expr ")";
```

## Grouping

Items can be grouped using parentheses.

```
program ::= (statement ";")*;
```

## Unions

A rule can be made to match one item or another using `|` between them.

```
program ::= program statement | statement;
```

`|` can be used within parentheses.

```
list ::= list ("," | ";") list_item | list_item;
```

Empty alternatives in a `|` expression have no effect. The `?` and `*` operators are the only way to make a rule optional.

```
# Equivalent of 'if_statement | expr ";"'
statement ::=
  | if_statement
  | expr ";"
  ;
```

## Repetition
`+` means that the previous rule, token, literal, or group must be matched one or more times.

```
program ::= statement+;
```

`*` requires the previous rule, token, literal, or group to be matched zero or more times.

```
list ::= (list_item ",")* list_item;
```

## Optional

`?` means that the previous previous rule, token, literal, or group can be matched one or zero times.

```
list ::= ((list_item ",")* list_item)?;
```
