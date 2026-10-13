; examples/light.paka, written by hand to docs/abi/m1.md.
target triple = "x86_64-pc-linux-gnu"

@name = private constant [5 x i8] c"\6C\69\67\68\74"
@s0 = private constant [18 x i8] c"\6C\65\74\20\74\68\65\72\65\20\62\65\20\6C\69\67\68\74"
@fx = private constant [2 x { i64, ptr, i64 }] [
  { i64, ptr, i64 } { i64 0, ptr @s0, i64 18 },
  { i64, ptr, i64 } { i64 1, ptr null, i64 0 } ]

declare i32 @paka_rt1_run(ptr, i64, ptr, i64, i64)

define internal i32 @p_update(i64 %m, ptr %out) {
  store { ptr, i64 } { ptr @fx, i64 2 }, ptr %out
  ret i32 0
}

define i32 @main() {
  %st = call i32 @paka_rt1_run(ptr @name, i64 5, ptr @p_update, i64 0, i64 1)
  ret i32 %st
}
