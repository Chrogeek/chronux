/*!
为了满足您的需求（点在直线两端速度为零、中间已知时间点的位置、整个运动持续时间已知、插值结果速度非负且连续），我推荐使用**单调三次 Hermite 插值（Monotonic Cubic Hermite Interpolation）**，具体实现上类似于 PCHIP（Piecewise Cubic Hermite Interpolating Polynomial）方法。这种方法能确保位置函数 \( p(t) \) 在整个时间区间 \([0, T]\) 上：
- 通过所有已知点（包括起点和终点）。
- 速度 \( v(t) = \frac{dp}{dt} \) 连续（因为使用分段三次多项式，且节点处导数匹配）。
- 速度非负（\( v(t) \geq 0 \)），即位置单调非减，不会出现后退。
- 在起点和终点强制速度为零（\( v(0) = 0 \) 和 \( v(T) = 0 \)）。

以下详细描述算法设计。假设输入数据合理：
- 总持续时间 \( T > 0 \)。
- 已知时间点 \( t_0, t_1, \dots, t_M \)（包括起点 \( t_0 = 0 \) 和终点 \( t_M = T \)，以及中间点 \( t_1 \) 到 \( t_{M-1} \))，时间点严格递增（无重复），即 \( 0 = t_0 < t_1 < \dots < t_M = T \)。
- 对应位置 \( p_0, p_1, \dots, p_M \)（包括起点 \( p_0 = 0 \) 和终点 \( p_M = P \)，中间点 \( p_i \) 满足 \( p_0 \leq p_1 \leq \dots \leq p_M \)，即位置非减）。

### 插值算法步骤
算法核心是为每个已知点 \( (t_i, p_i) \) 计算一个导数（速度）\( m_i \)，然后使用分段三次 Hermite 插值构建位置函数 \( p(t) \)。导数 \( m_i \) 的选择是关键，它确保每个区间内插值单调且速度连续。

1. **预处理输入数据**：
   - 令时间点序列为 \( (t_i, p_i) \) 其中 \( i = 0, 1, \dots, M \)，满足 \( t_0 = 0 \), \( t_M = T \), \( p_0 = 0 \), \( p_M = P \)。
   - 对于每个区间 \( i = 0, 1, \dots, M-1 \)，计算：
     - 区间长度： \( h_i = t_{i+1} - t_i \)（由于时间点严格递增，\( h_i > 0 \))。
     - 平均斜率： \( d_i = \frac{p_{i+1} - p_i}{h_i} \)（由于位置非减，\( d_i \geq 0 \))。

2. **设置节点导数（速度）\( m_i \)**：
   - 起点： \( m_0 = 0 \)（强制起点速度为零）。
   - 终点： \( m_M = 0 \)（强制终点速度为零）。
   - 对于内部点 \( i = 1, 2, \dots, M-1 \):
     - 如果 \( d_{i-1} = 0 \) 或 \( d_i = 0 \)（即点 \( i \) 处有平坦段），则设 \( m_i = 0 \)（避免非单调行为）。
     - 否则（即 \( d_{i-1} > 0 \) 和 \( d_i > 0 \)，点 \( i \) 处严格递增），设 \( m_i = \frac{2 \cdot d_{i-1} \cdot d_i}{d_{i-1} + d_i} \)（调和平均数，确保单调性）。
     - 注意：由于输入位置非减，\( d_i \geq 0 \)，因此无需处理斜率符号变化。

3. **构建分段三次 Hermite 插值函数**：
   - 对于每个区间 \( [t_i, t_{i+1}] \)，其中 \( i = 0, 1, \dots, M-1 \)，定义三次多项式：
     \[
     p(t) = p_i + m_i (t - t_i) + c_2 (t - t_i)^2 + c_3 (t - t_i)^3, \quad t \in [t_i, t_{i+1}]
     \]
     系数 \( c_2 \) 和 \( c_3 \) 由以下条件计算（确保在区间端点处位置和导数连续）：
     - \( p(t_i) = p_i \)
     - \( p(t_{i+1}) = p_{i+1} \)
     - \( p'(t_i) = m_i \)
     - \( p'(t_{i+1}) = m_{i+1} \)

     具体系数公式为：
     \[
     \begin{align*}
     c_2 &= \frac{3d_i - 2m_i - m_{i+1}}{h_i} \\
     c_3 &= \frac{m_i + m_{i+1} - 2d_i}{h_i^2}
     \end{align*}
     \]
     其中 \( d_i = \frac{p_{i+1} - p_i}{h_i} \) 是区间平均斜率。

4. **插值函数 \( p(t) \) 的定义**：
   - 对于任意时间 \( t \in [0, T] \)，首先找到 \( t \) 所在的区间 \( [t_i, t_{i+1}] \)（即满足 \( t_i \leq t \leq t_{i+1} \) 的 \( i \))。
   - 然后使用该区间对应的三次多项式计算位置 \( p(t) \)。

5. **速度函数 \( v(t) \) 的获取**：
   - 在区间 \( [t_i, t_{i+1}] \) 内，速度函数为：
     \[
     v(t) = m_i + 2c_2 (t - t_i) + 3c_3 (t - t_i)^2
     \]
     - 由于算法设计，在节点处 \( v(t_i) = m_i \) 和 \( v(t_{i+1}) = m_{i+1} \) 匹配，因此整个 \( v(t) \) 连续。
     - 通过导数 \( m_i \) 的选择和系数计算，可证明 \( v(t) \geq 0 \) 对于所有 \( t \)（见下文保证）。

### 算法保证和性质
- **速度非负 (\( v(t) \geq 0 \))**：通过设置 \( m_i \)（特别是使用调和平均和零导数处理平坦段），并结合三次 Hermite 插值的单调性条件，确保每个区间内 \( p(t) \) 单调非减，因此 \( v(t) \geq 0 \)。在起点和终点，\( v(0) = m_0 = 0 \) 和 \( v(T) = m_M = 0 \)，符合要求。
- **速度连续**：由于每个区间内 \( p(t) \) 是三次多项式，\( v(t) \) 是二次多项式（连续）；在节点处，左导数和右导数都等于 \( m_i \)，因此 \( v(t) \) 在整个 \([0, T]\) 上连续。
- **通过已知点**：插值函数精确通过所有输入点 \( (t_i, p_i) \)。
- **计算效率**：算法复杂度为 \( O(M) \) 预处理（计算 \( h_i, d_i, m_i \))，插值时 \( O(1) \) 每点（通过区间搜索）。
- **边界处理**：起点和终点速度显式设为零，确保平滑启停。

### 为什么选择此方法？
- **PCHIP 类似**：此算法类似于 PCHIP，专为形状保持（shape-preserving）设计，特别适合单调数据。相比标准三次样条，它避免过冲和非单调行为。
- **输入假设利用**：输入位置非减，因此 \( d_i \geq 0 \)，简化了导数计算（无需处理负斜率）。
- **速度零边界**：显式设置 \( m_0 = 0 \) 和 \( m_M = 0 \) 满足边界条件。

### 实现建议
- **编程实现**：
  - 预处理：计算所有 \( h_i \), \( d_i \), 和 \( m_i \)。
  - 插值函数：对于给定 \( t \)，二分查找定位区间，然后计算多项式。
  - 示例 Python 伪代码：
    ```python
    import numpy as np

    def monotonic_cubic_interp(t_points, p_points, T):
        # 输入: t_points = [t0, t1, ..., tM], p_points = [p0, p1, ..., pM]
        # 假设 t0=0, tM=T, p0=0, pM=P, 点已排序
        M = len(t_points) - 1  # 区间数 (点数为 M+1)
        h = [t_points[i+1] - t_points[i] for i in range(M)]
        d = [(p_points[i+1] - p_points[i]) / h[i] for i in range(M)]

        # 设置导数 m_i
        m = [0.0] * (M+1)  # 初始化
        m[0] = 0.0  # 起点速度零
        m[M] = 0.0  # 终点速度零
        for i in range(1, M):
            if d[i-1] == 0 or d[i] == 0:
                m[i] = 0.0
            else:
                m[i] = 2 * (d[i-1] * d[i]) / (d[i-1] + d[i])  # 调和平均

        # 构建插值函数
        def p(t):
            # 找到 t 所在区间
            i = np.searchsorted(t_points, t, side='right') - 1
            if i < 0:
                i = 0
            if i >= M:
                i = M - 1
            # 计算区间内相对时间和系数
            dt = t - t_points[i]
            c2 = (3 * d[i] - 2 * m[i] - m[i+1]) / h[i]
            c3 = (m[i] + m[i+1] - 2 * d[i]) / (h[i] ** 2)
            return p_points[i] + m[i] * dt + c2 * dt**2 + c3 * dt**3

        return p
    ```
- **现成库使用**：在实际应用中，可直接使用科学计算库：
  - Python: `scipy.interpolate.PchipInterpolator`（设置 `extrapolate=False` 并确保边界导数为零）。
  - MATLAB: `pchip` 函数。
  - 注意：标准 PCHIP 可能不显式设边界导数为零，需手动调整（例如，在 `scipy` 中，可通过指定 `derivatives` 参数）。

### 潜在边界情况处理
- **平坦段（\( d_i = 0 \))**：算法设相关 \( m_i = 0 \)，确保区间内位置恒定，速度为零。
- **线性段**：如果数据近似线性（\( d_i \) 常数），调和平均使 \( m_i \approx d_i \)，插值接近线性。
- **密集点**：方法对输入点密度不敏感，但点过少时，插值可能不够平滑（仍满足速度非负和连续）。

此方法平衡了简单性和鲁棒性，严格满足您的要求。如果您有具体数据或实现问题，我可以进一步调整算法细节。
 */

use crate::util::{ChronuxError, ChronuxResult};

#[derive(Debug, Clone)]
pub struct PchipInterpolator {
    t: Vec<f32>,
    p: Vec<f32>,
    m: Vec<f32>,
    // h: Vec<f32>,
    // d: Vec<f32>,
    c2: Vec<f32>,
    c3: Vec<f32>,
    max_t: f32,
    n: usize,
    t_start: f32,
    p_start: f32,
    p_end: f32,
}

impl PchipInterpolator {
    pub fn new(t: Vec<f32>, p: Vec<f32>) -> ChronuxResult<PchipInterpolator> {
        // check input validity
        if t.len() != p.len() || t.is_empty() {
            return Err(ChronuxError::ArgError(
                "Time and position arrays must have the same non-zero length.".into(),
            ));
        }

        let n = t.len();

        // check if t is strictly increasing
        for i in 1..n {
            if t[i] <= t[i - 1] {
                return Err(ChronuxError::ArgError(
                    "Time points must be strictly increasing.".into(),
                ));
            }
        }

        // check if p is non-decreasing
        for i in 1..n {
            if p[i] < p[i - 1] {
                return Err(ChronuxError::ArgError(
                    "Position points must be non-decreasing.".into(),
                ));
            }
        }

        if p[n - 1] <= p[0] {
            return Err(ChronuxError::ArgError(
                "End position must be greater than start position.".into(),
            ));
        }

        let t_start = t[0];
        let max_t = *t.last().unwrap() - t_start;
        let p_start = p[0];
        let p_end = p[n - 1];

        let t = t.iter().map(|v| v - t_start).collect::<Vec<_>>();
        let p = p
            .iter()
            .map(|v| (v - p_start) / (p_end - p_start))
            .collect::<Vec<_>>();

        let h = t.windows(2).map(|w| w[1] - w[0]).collect::<Vec<_>>();
        let d = p
            .windows(2)
            .zip(&h)
            .map(|(w, h)| (w[1] - w[0]) / h)
            .collect::<Vec<_>>();
        let mut m = d
            .windows(2)
            .map(|w| 2.0 * (w[0] * w[1]) / (w[0] + w[1]))
            .fold(vec![0.0], |mut acc, v| {
                acc.push(v);
                acc
            });
        m.push(0.0); // m = 0 for the last point

        let c2 = (0..n - 1)
            .map(|i| (3.0 * d[i] - 2.0 * m[i] - m[i + 1]) / h[i])
            .collect::<Vec<_>>();
        let c3 = (0..n - 1)
            .map(|i| (m[i] + m[i + 1] - 2.0 * d[i]) / (h[i] * h[i]))
            .collect::<Vec<_>>();

        Ok(PchipInterpolator {
            t,
            p,
            m,
            // h,
            // d,
            c2,
            c3,
            max_t,
            n,
            t_start,
            p_start,
            p_end,
        })
    }

    pub fn interpolate(&self, t: f32) -> Option<f32> {
        if t <= self.t_start || t >= self.max_t + self.t_start {
            log::warn!(
                "Time point out of bounds in PCHIP interpolation: t = {}, t_start = {}, t_length = {}",
                t,
                self.t_start,
                self.max_t
            );
            return None;
        }
        let t = t - self.t_start;
        let i = self.t.iter().position(|&x| x > t).unwrap_or(self.n - 1) - 1;
        let dt = t - self.t[i];
        let p_i = self.p[i];
        let m_i = self.m[i];
        let c2_i = self.c2[i];
        let c3_i = self.c3[i];
        let p = p_i + m_i * dt + c2_i * dt * dt + c3_i * dt * dt * dt;
        Some(p * (self.p_end - self.p_start) + self.p_start)
    }
}
