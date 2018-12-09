use event::User;
use rlua::{Function, Lua, UserData, UserDataMethods};

impl UserData for User {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("name", |_, this, _: ()| Ok(this.username.0.clone()));
        methods.add_method("email", |_, this, _: ()| Ok(this.email.0.clone()));
        methods.add_method("ip", |_, this, _: ()| Ok(this.ip.0.clone()));
        methods.add_method("ua", |_, this, _: ()| Ok(this.user_agent.0.clone()));
        methods.add_method("fp", |_, this, _: ()| match this.finger_print {
            Some(ref fp) => Ok(fp.0.clone()),
            None => Ok(String::from("<NO PRINT>")),
        });
    }
}

pub fn new_lua() -> Lua {
    Lua::new()
}

pub fn call_constraints_function(rule: &str, user: User, l: &Lua) -> Result<bool, rlua::Error> {
    let f: Function = l.eval(&("function(user) return ".to_owned() + rule + " end"), None)?;
    let v: bool = f.call::<_, bool>(user)?;
    Ok(v)
}
