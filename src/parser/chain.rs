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

use std::{collections::HashMap, fmt::Display};

use serde::Serialize;

#[derive(Debug, Clone)]
pub enum ChainedPath {
    Atom(String),
    Seq(Vec<ChainedPath>),
    Par(Vec<ChainedPath>),
    Rev(Box<ChainedPath>),
}

impl ChainedPath {
    // pub fn is_atomic(&self) -> bool {
    //     matches!(self, ChainedPath::Atom(_))
    // }

    pub fn get_atomic(&self) -> Option<&String> {
        if let ChainedPath::Atom(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn new(raw: &str) -> ChainedPath {
        fn split_args(raw: &str) -> Vec<&str> {
            let mut l = 0;
            let mut nest = 0;
            let mut ret = vec![];
            for (r, c) in raw.as_bytes().iter().enumerate() {
                match c {
                    b'(' => nest += 1,
                    b')' => nest -= 1,
                    b',' if nest == 0 => {
                        ret.push(&raw[l..r]);
                        l = r + 1;
                    }
                    _ => continue,
                }
            }
            if l < raw.len() {
                ret.push(&raw[l..]);
            }
            return ret;
        }

        let raw = raw.trim();
        if raw.starts_with("#rev(") && raw.ends_with(")") {
            let inner = &raw[5..raw.len() - 1];
            let child = Self::new(inner);
            ChainedPath::Rev(Box::new(child))
        } else if raw.starts_with("#seq(") && raw.ends_with(")") {
            let inner = &raw[5..raw.len() - 1];
            let parts = split_args(inner);
            let paths = parts.iter().map(|s| Self::new(s)).collect();
            ChainedPath::Seq(paths)
        } else if raw.starts_with("#par(") && raw.ends_with(")") {
            let inner = &raw[5..raw.len() - 1];
            let parts = split_args(inner);
            let paths = parts.iter().map(|s| Self::new(s)).collect();
            ChainedPath::Par(paths)
        } else {
            ChainedPath::Atom(raw.to_string())
        }
    }

    pub fn flatten(&self) -> Vec<String> {
        match self {
            ChainedPath::Atom(atom) => vec![atom.clone()],
            ChainedPath::Seq(chained_paths) | ChainedPath::Par(chained_paths) => chained_paths
                .iter()
                .map(|child| child.flatten())
                .flatten()
                .collect(),
            ChainedPath::Rev(sub) => sub.flatten(),
        }
    }

    // pub fn contains_object(&self, obj: &str) -> bool {
    //     match self {
    //         ChainedPath::Atom(o) => o == obj,
    //         ChainedPath::Seq(chained_paths) | ChainedPath::Par(chained_paths) => {
    //             chained_paths.iter().any(|c| c.contains_object(obj))
    //         }
    //     }
    // }

    pub fn length(&self, dict: &HashMap<String, f32>) -> Option<f32> {
        match self {
            ChainedPath::Atom(o) => dict.get(o).cloned(),
            ChainedPath::Seq(chained_paths) => {
                chained_paths
                    .iter()
                    .map(|o| o.length(dict))
                    .fold(Some(0.0), |acc, len_opt| match (acc, len_opt) {
                        (Some(acc_len), Some(len)) => Some(acc_len + len),
                        _ => None,
                    })
            }
            ChainedPath::Par(chained_paths) => chained_paths
                .iter()
                .map(|o| o.length(dict))
                .fold(Some(0.0), |acc, len_opt| match (acc, len_opt) {
                    (Some(acc_len), Some(len)) => Some(acc_len + len),
                    _ => None,
                })
                .map(|len| len / (chained_paths.len() as f32)),
            ChainedPath::Rev(sub) => sub.length(dict),
        }
    }

    // 定位 atom 路径在整个 chained_path 上的起止位置，返回 Option<(start, end)>
    pub fn locate(&self, atom: &str, dict: &HashMap<String, f32>) -> Option<(f32, f32)> {
        match self {
            ChainedPath::Atom(o) => {
                if o == atom {
                    dict.get(o).cloned().map(|len| (0.0, len))
                } else {
                    None
                }
            }
            ChainedPath::Seq(chained_paths) => {
                let mut accumulated_length = 0.0;
                for child in chained_paths {
                    if let Some((start, end)) = child.locate(atom, dict) {
                        return Some((accumulated_length + start, accumulated_length + end));
                    }
                    accumulated_length += child.length(dict)?;
                }
                None
            }
            ChainedPath::Par(chained_paths) => {
                let lengths = chained_paths
                    .iter()
                    .map(|c| c.length(dict))
                    .collect::<Option<Vec<_>>>()?;
                let avg_length = lengths.iter().sum::<f32>() / (chained_paths.len() as f32);
                for (id, child) in chained_paths.iter().enumerate() {
                    if let Some((start, end)) = child.locate(atom, dict) {
                        let scale = avg_length / lengths[id];
                        return Some((start * scale, end * scale));
                    }
                }
                None
            }
            ChainedPath::Rev(sub) => {
                if let Some((start, end)) = sub.locate(atom, dict) {
                    let total_length = sub.length(dict)?;
                    Some((total_length - end, total_length - start))
                } else {
                    None
                }
            }
        }
    }
}

impl From<&str> for ChainedPath {
    fn from(raw: &str) -> Self {
        ChainedPath::new(raw)
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
            ChainedPath::Rev(sub) => {
                write!(f, "#rev({})", sub)
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
        Ok(ChainedPath::new(&s))
    }
}
