/*!

# Chained trackers
将多个路径连接在一起（支持串联 #seq、并联 #par）。
每个路径仍然保持自己的属性，但是在计算路径长度、执行动画时会视为一个整体。

## 语法
```plain
#seq(path1, path2, ...)
#par(path1, path2, ...)
```

支持嵌套，即参数 path 可以是另一个 #seq 或 #par。

*/

use std::fmt::Display;

use serde::Serialize;

#[derive(Debug, Clone)]
pub enum ChainedPath {
    Atom(String),
    Seq(Vec<ChainedPath>),
    Par(Vec<ChainedPath>),
}

impl ChainedPath {
    pub fn is_atomic(&self) -> bool {
        matches!(self, ChainedPath::Atom(_))
    }

    pub fn get_atomic(&self) -> Option<&String> {
        if let ChainedPath::Atom(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn from_raw(raw: &str) -> ChainedPath {
        let raw = raw.trim();
        if raw.starts_with("#seq(") && raw.ends_with(")") {
            let inner = &raw[5..raw.len() - 1];
            let parts = str::split(inner, ',');
            let paths = parts.map(|s| Self::from_raw(s)).collect();
            ChainedPath::Seq(paths)
        } else if raw.starts_with("#par(") && raw.ends_with(")") {
            let inner = &raw[5..raw.len() - 1];
            let parts = str::split(inner, ',');
            let paths = parts.map(|s| Self::from_raw(s)).collect();
            ChainedPath::Par(paths)
        } else {
            ChainedPath::Atom(raw.to_string())
        }
    }
}

impl Display for ChainedPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainedPath::Atom(s) => write!(f, "{}", s),
            ChainedPath::Seq(paths) => {
                let parts: Vec<String> = paths.iter().map(|p| format!("{}", p)).collect();
                write!(f, "#seq({})", parts.join(", "))
            }
            ChainedPath::Par(paths) => {
                let parts: Vec<String> = paths.iter().map(|p| format!("{}", p)).collect();
                write!(f, "#par({})", parts.join(", "))
            }
        }
    }
}

impl Serialize for ChainedPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for ChainedPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(ChainedPath::from_raw(&s))
    }
}
