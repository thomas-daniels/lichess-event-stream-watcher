use event::User;
use regex::Regex;
use rlua;
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
    let l = Lua::new();
    l.context(|lua_ctx| {
        let regex_fn = lua_ctx
            .create_function(
                |_, (text, pattern): (String, String)| match Regex::new(&pattern) {
                    Ok(re) => Ok(re.is_match(&text)),
                    Err(_) => Err(rlua::Error::RuntimeError(String::from(
                        "Error in 'regex' function",
                    ))),
                },
            )
            .unwrap();
        let globals = lua_ctx.globals();
        globals.set("regex", regex_fn).unwrap();
    });
    l
}

pub fn call_constraints_function(rule: &str, user: User, l: &Lua) -> Result<bool, rlua::Error> {
    let mut v: bool = false;
    l.context(|lua_ctx| {
        let f: Function = lua_ctx.load(&("function(user) return ".to_owned() + rule + " end")).eval()?;
        v = f.call::<_, bool>(user)?;
        Ok(())
    })?;
    Ok(v)
}
