use std::collections::HashMap;
use std::path::PathBuf;

use derive_more::{Deref, DerefMut};

use crate::content::{Page, Section};

#[derive(Default, Deref, DerefMut)]
pub struct Sections(HashMap<PathBuf, Section>);

#[derive(Default, Deref, DerefMut)]
pub struct Pages(HashMap<PathBuf, Page>);
