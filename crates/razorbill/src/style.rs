pub fn plumage() -> StyleBuilder {
    StyleBuilder {
        classes: Vec::new(),
    }
}

pub struct StyleBuilder {
    classes: Vec<String>,
}

impl From<StyleBuilder> for String {
    fn from(value: StyleBuilder) -> Self {
        value.classes.join(" ")
    }
}

macro_rules! style_methods {
    ($($method_name:ident : $class_name:expr),*) => {
        $(
            pub fn $method_name(self) -> Self {
                self.class($class_name)
            }
        )*
    }
}

impl StyleBuilder {
    #[inline(always)]
    pub fn class(mut self, class: impl Into<String>) -> Self {
        self.classes.push(class.into());
        self
    }

    style_methods! {
        p_0 : "pa0",
        p_1 : "pa1",
        p_2 : "pa2",
        p_3 : "pa3",
        p_4 : "pa4",
        p_5 : "pa5",
        p_6 : "pa6",
        p_7 : "pa7"
    }

    style_methods! {
        pl_0 : "pl0",
        pl_1 : "pl1",
        pl_2 : "pl2",
        pl_3 : "pl3",
        pl_4 : "pl4",
        pl_5 : "pl5",
        pl_6 : "pl6",
        pl_7 : "pl7"
    }

    style_methods! {
        pr_0 : "pr0",
        pr_1 : "pr1",
        pr_2 : "pr2",
        pr_3 : "pr3",
        pr_4 : "pr4",
        pr_5 : "pr5",
        pr_6 : "pr6",
        pr_7 : "pr7"
    }

    style_methods! {
        pt_0 : "pt0",
        pt_1 : "pt1",
        pt_2 : "pt2",
        pt_3 : "pt3",
        pt_4 : "pt4",
        pt_5 : "pt5",
        pt_6 : "pt6",
        pt_7 : "pt7"
    }

    style_methods! {
        pb_0 : "pb0",
        pb_1 : "pb1",
        pb_2 : "pb2",
        pb_3 : "pb3",
        pb_4 : "pb4",
        pb_5 : "pb5",
        pb_6 : "pb6",
        pb_7 : "pb7"
    }

    style_methods! {
        px_0 : "ph0",
        px_1 : "ph1",
        px_2 : "ph2",
        px_3 : "ph3",
        px_4 : "ph4",
        px_5 : "ph5",
        px_6 : "ph6",
        px_7 : "ph7"
    }

    style_methods! {
        py_0 : "pv0",
        py_1 : "pv1",
        py_2 : "pv2",
        py_3 : "pv3",
        py_4 : "pv4",
        py_5 : "pv5",
        py_6 : "pv6",
        py_7 : "pv7"
    }

    style_methods! {
        text_left : "tl",
        text_right : "tr",
        text_center : "tc",
        text_justify : "tj"
    }
}
