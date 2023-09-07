use event::User;
use regex::Regex;
use rlua;
use rlua::{Function, Lua, UserData, UserDataMethods};
use std::net::IpAddr;

impl UserData for User {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("name", |_, this, _: ()| Ok(this.username.0.clone()));
        methods.add_method("email", |_, this, _: ()| Ok(this.email.0.clone()));
        methods.add_method("ip", |_, this, _: ()| Ok(this.ip.0.clone()));
        methods.add_method("ua", |_, this, _: ()| match this.user_agent {
            Some(ref ua) => Ok(ua.0.clone()),
            None => Ok(String::from("no user agent detected")),
        });
        methods.add_method("fp", |_, this, _: ()| match this.finger_print {
            Some(ref fp) => Ok(fp.0.clone()),
            None => Ok(String::from("<NO PRINT>")),
        });
        methods.add_method("country", |_, this, _: ()| {
            Ok(this
                .geoip
                .as_ref()
                .and_then(|g| g.country.clone())
                .unwrap_or(String::from("<NO COUNTRY>")))
        });
        methods.add_method("city", |_, this, _: ()| {
            Ok(this
                .geoip
                .as_ref()
                .and_then(|g| g.city.clone())
                .unwrap_or(String::from("<NO CITY>")))
        });
        methods.add_method("subdivisions", |_, this, _: ()| {
            Ok(this
                .geoip
                .as_ref()
                .and_then(|g| g.subdivisions.clone())
                .unwrap_or(vec![]))
        });
        methods.add_method("has_subdivision", |_, this, args: (String,)| {
            Ok(this
                .geoip
                .as_ref()
                .and_then(|g| g.subdivisions.as_ref().map(|s| s.contains(&args.0))))
        });
        methods.add_method("device", |_, this, _: ()| {
            Ok(this
                .device
                .as_ref()
                .map(|d| d.device.clone())
                .unwrap_or(String::from("<NO DEVICE>")))
        });
        methods.add_method("os", |_, this, _: ()| {
            Ok(this
                .device
                .as_ref()
                .map(|d| d.os.clone())
                .unwrap_or(String::from("<NO OS>")))
        });
        methods.add_method("client", |_, this, _: ()| {
            Ok(this
                .device
                .as_ref()
                .map(|d| d.client.clone())
                .unwrap_or(String::from("<NO CLIENT>")))
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
        let is_in_ip_range = lua_ctx
            .create_function(|_, (ip, min, max): (String, String, String)| {
                let addr = ip.parse();
                let min_addr = min.parse();
                let max_addr = max.parse();
                if addr.is_err() || min_addr.is_err() || max_addr.is_err() {
                    Err(rlua::Error::RuntimeError(String::from(
                        "Invalid IP in one of the arguments",
                    )))
                } else {
                    let addr: IpAddr = addr.unwrap();
                    let min_addr: IpAddr = min_addr.unwrap();
                    let max_addr: IpAddr = max_addr.unwrap();
                    Ok(addr >= min_addr && addr <= max_addr)
                }
            })
            .unwrap();
        let globals = lua_ctx.globals();
        globals.set("regex", regex_fn).unwrap();
        globals.set("isInIpRange", is_in_ip_range).unwrap();
    });
    l
}

pub fn call_constraints_function(rule: &str, user: User, l: &Lua) -> Result<bool, rlua::Error> {
    let mut v: bool = false;
    l.context(|lua_ctx| {
        let f: Function = lua_ctx
            .load(&("function(user) return ".to_owned() + rule + " end"))
            .eval()?;
        v = f.call::<_, bool>(user)?;
        Ok(())
    })?;
    Ok(v)
}
