# RolScript   
RolScript是一门由Rust实现的轻量级动态脚本语言。   

## 特性
- 强类型
- 模块
- 垃圾回收
- 闭包
- lambda表达式
- if表达式
- 块表达式
- 自定义类型
- 可重载运算符

## 内置类型
- Null
- Bool
- Int
- Float
- String
- Tuple
- Array
- Map
- Function
- Option
- Module

## 语法
```
# '#'开头的是注释

# 算数表达式
a = 1 + 2 * 3;
# 函数调用
print(a);       # => 7

# 内置类型
b = true;
num = 1;
str = "hello world!";
tup = (1,2,3,4);
arr = [1,2,3,4];

# 块表达式，用大括号包裹的一系列语句和表达式。
a = {
    aa = 1 + 2 * 3;
    aa
}
print(a);       # => 7

a = 1;
b = 2;

# if语句，当内部只有一条语句或表达式的时候，可以省略大括号。
if (a < b) {
    print("a < b");
} else 
    print("a > b");

# if也可以当作表达式使用。
c = if (a < b) {
    "a < b"
} else {
    "a > b"
}
print(c);        # => "a < b"

# while语句
i = 0;
while (i < 4) {
    print(i);
    i = i + 1;
}
# 输出：
# 0
# 1
# 2 
# 3 

# for语句，用于遍历可迭代对象。
for (n : [1,2,3,4])
    print(n);
# 输出：
# 1
# 2 
# 3 
# 4

# 函数定义，可以递归。
function flb(n) {
    if (n <= 2)
        return 1;
    else
        return flb(n - 2) + flb(n - 1);
}
print(flb(16)); # => 987

# lambda表达式
f = (n) => n * 2;
print(f(2));    # => 4

# 自定义类型
type A {
    # 构造函数
    function [new](arg1, arg2) {
        # ...
    }

    # 方法
    function method1(arg1, arg2) {
        # ...
    }

    # 重载加法运算符
    function +(other) {
        # ...
    }

    # 重载可迭代运算符，该运算符应返回一个迭代器。
    # 重载此运算符后便成为可迭代对象。
    function [iter]() {
        # ...
    }

    # 重载迭代器运算符，类似于rust的迭代器。
    # 重载此运算符后便成为迭代器对象。
    function [next]() {
        # ...
    }
}

# 创建自定义类型的实例。
a = A();

```

## 使用
```
>> git clone ...
>> cd ...
>> cargo build
>> cargo run
```

## 项目结构
```

```

## 警告
本项目尚未完成，且没有进行严格的测试，仅用于个人学习目的。

## License
MIT
