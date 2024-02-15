// 定义模块mod_a
mod mod_a {
    // 在mod_a内部定义结构体StructB
    pub struct StructB {
        // StructB包含一个公共字段field_c
        pub field_c: i32,
    }

    fn foo() {
        let a=10;
        let mut b:bool = false;
    }
}

fn main() {
    // 创建StructB的实例，并初始化field_c
    let mut instance_b = mod_a::StructB { field_c: 10 };
    instance_b = mod_a::StructB { field_c: 11 };

    // 访问field_c字段
    println!("field_c value: {}", instance_b.field_c);
}
