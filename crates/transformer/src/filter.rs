pub struct IdentItem<'a> {
    pub ident: &'a str,
    pub range: (isize, isize),
}

pub trait IdentFilterPlugin {
    fn filter_ident(&self, ident: &IdentItem<'_>) -> bool;
}

pub struct IdentFilterPluginAdapter {
    plugin: Vec<Box<dyn IdentFilterPlugin>>,
}

impl IdentFilterPluginAdapter {
    pub fn new(plugin: Vec<Box<dyn IdentFilterPlugin>>) -> Self {
        Self { plugin }
    }

    pub fn with_plugin(mut self, plugin: Box<dyn IdentFilterPlugin>) -> Self {
        self.plugin.push(plugin);
        self
    }
}

impl IdentFilterPlugin for IdentFilterPluginAdapter {
    fn filter_ident(&self, ident: &IdentItem<'_>) -> bool {
        self.plugin.iter().any(|v| v.filter_ident(ident))
    }
}
