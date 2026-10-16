//! A dense revised simplex method with a partitioned basis inverse, for the master LP of a Dantzig–Wolfe
//! decomposition.  Research code for `examples/fields.rs` (`#[path = "common/lp.rs"] mod lp;`); it knows nothing of
//! matroids.
//!
//! # The LP
//!
//! Maximize `c·x` subject to `A x (= or ≤) b`, `x ≥ 0`.  An `Eq` row has a free dual; a `Le` row has a slack
//! `s_r ≥ 0` and a dual `≥ 0` at an optimum.  The caller keeps the LP bounded (in the master every column has an
//! entry 1 in one `Eq` row of rhs 1, a convexity row, though nothing here relies on that beyond boundedness) and
//! hands over primal feasible bases.
//!
//! # The partitioned basis
//!
//! The master has tens of `Eq` rows, thousands of `Le` rows and up to 10⁵ columns, and most `Le` rows keep their
//! slack basic throughout.  So the rows are split into *S-rows*, the `Le` rows whose slack is basic, and *C-rows*,
//! the `Eq` rows and the `Le` rows whose slack is nonbasic.  A basis has as many structural columns as there are
//! C-rows, `s` of each: there are as many basic variables as rows, and the S-rows' slacks are the rest.  `B_s` is
//! the `s × s` submatrix of `A` on the C-rows and the structural basic columns, and with the rows ordered C then S
//! and the basic columns structural then slack, `B = [[B_s, 0], [N, I]]` (`N`: the structural basic columns'
//! entries in the S-rows; a slack is a unit column on its own row), so `B⁻¹ = [[B_s⁻¹, 0], [−N B_s⁻¹, I]]`, as
//! multiplying out shows.  Only `Binv = B_s⁻¹` is stored, dense and row-major, its rows indexed by structural
//! position `p` and its columns by C-position `q`; the S-rows are reached through the sparse columns.  Then
//!
//! * the basic values `x_B = B⁻¹ b` are `x_struct = Binv b_C` and, for an S-row `r`, the slack
//!   `b_r − Σ_p A[r][scol[p]] x_struct[p]`;
//! * the duals `y = c_B B⁻¹`, with `c_B = (c_struct, 0)`, are `y_C = c_struct Binv` and `y_r = 0` on the S-rows;
//! * the reduced cost `c_j − y·a_j` of a structural column is `d_j = c_j − Σ_{r ∈ C} y_r A[r][j]`, and of the slack
//!   of a C-row `r` (column `e_r`, cost 0) it is `−y_r`; a variable may enter when it is above the tolerance;
//! * FTRAN of an entering column `a` is `B⁻¹ a = (u, v)` with `u = Binv a_C` and, for an S-row `r`,
//!   `v_r = a_r − Σ_p A[r][scol[p]] u_p`;
//! * the ratio test runs over `x_struct[p]` with `u_p > 0` and the slacks of the S-rows with `v_r > 0`.
//!
//! A pivot changes `B_s` by a column (a structural column replaces another), by a border (a structural column
//! enters and an S-row's slack leaves), by losing a row and a column (a C-row's slack enters and a structural column
//! leaves), or by a row (a C-row's slack enters and an S-row's slack leaves); each is an `O(s²)` update of `Binv`,
//! derived at the method that makes it.  A pivot costs that, the S-row entries of the structural basic columns
//! (FTRAN), and the pricing pass over the live columns' entries.
//!
//! # The rules
//!
//! [`Lp::solve`] is the primal simplex method: Dantzig's rule, Harris's two-pass ratio test, Bland's rule after a
//! run of degenerate pivots, and a fresh `Binv` by Gauss–Jordan elimination every 64 pivots.

/// A reduced cost above this may enter.
const PRICE_TOL: f64 = 1e-9;
/// Harris's tolerance: the ratio test's relaxed bound lets a basic value fall to −`FEAS_TOL`.
const FEAS_TOL: f64 = 1e-9;
/// An entry of `B⁻¹ a` at most this is never a pivot.
const PIVOT_TOL: f64 = 1e-9;
/// A step below this is degenerate.
const DEGENERATE: f64 = 1e-12;
/// After this many consecutive degenerate pivots, Bland's rule until a nondegenerate one.
const BLAND_AFTER: u32 = 50;
/// The shortest segment of columns a partial pricing scans (a segment is an eighth of the columns, or this).
const PARTIAL: usize = 200;
/// Pivots between refactorizations.
const REFACTOR_EVERY: u32 = 64;
/// Gauss–Jordan elimination's smallest pivot: below it the basis is singular.
const SINGULAR: f64 = 1e-11;
/// A basic value below this fails [`Lp::reset`] and an added row's slack below it fails [`Lp::add_row`].
const START_TOL: f64 = 1e-9;
/// A basic value below this fails [`Lp::feasible`].
const FEASIBLE_TOL: f64 = 1e-7;
/// No position: a nonbasic column, an S-row.
const NONE: u32 = u32::MAX;

/// The kind of a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// `a·x = b`, with a free dual.
    Eq,
    /// `a·x ≤ b`, with a slack; its dual is `≥ 0` at an optimum.
    Le,
}

/// How [`Lp::solve`] ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// No live column or slack has a reduced cost above the pricing tolerance.
    Optimal,
    /// The pivot limit came first.
    PivotLimit,
}

/// Counters.
#[derive(Clone, Copy, Debug, Default)]
pub struct LpStats {
    /// Pivots made.
    pub pivots: u64,
    /// Pivots with a step below `DEGENERATE`.
    pub degenerate: u64,
    /// Pivots chosen by Bland's rule.
    pub bland: u64,
    /// Refactorizations, those of [`Lp::reset`] and [`Lp::refactor`] included.
    pub refactors: u64,
    /// The largest number of C-rows.
    pub max_s: usize,
    /// Singular bases met at a periodic refactorization (rounding drift), each answered by a return to the last basis
    /// given to [`Lp::reset`].
    pub repairs: u64,
}

/// An entering variable: a structural column (by id), or the slack of a C-row (by row id).
#[derive(Clone, Copy)]
enum Enter {
    Column(usize),
    Slack(usize),
}

/// A leaving variable: a structural column (by position), or the slack of an S-row (by row id).
#[derive(Clone, Copy)]
enum Leave {
    Column(usize),
    Slack(usize),
}

/// The LP and its current basis in partitioned form (the module comment).
pub struct Lp {
    kind: Vec<RowKind>,
    rhs: Vec<f64>,
    /// Per row: its C-position, or `NONE` for an S-row.
    cpos: Vec<u32>,
    /// Per row: the value of its slack (`Le` rows; 0 when nonbasic).
    slack: Vec<f64>,
    /// Per row: its dual (0 on the S-rows).
    y: Vec<f64>,
    cost: Vec<f64>,
    /// Per column: its entries (row, value), sorted by row.  A row added later has the largest id, so pushing its
    /// entries keeps them sorted.
    cols: Vec<Vec<(u32, f64)>>,
    /// Per column: its structural position, or `NONE` when nonbasic.
    colpos: Vec<u32>,
    dead: Vec<bool>,
    /// Per C-position: its row.
    crow: Vec<u32>,
    /// Per structural position: its column, and the column's value.
    scol: Vec<u32>,
    xb: Vec<f64>,
    /// `B_s⁻¹`, row-major with row `p` at `binv[p * stride..p * stride + s]`.
    binv: Vec<f64>,
    stride: usize,
    /// False after an `Eq` row is added, until [`Lp::reset`].
    valid: bool,
    since_refactor: u32,
    /// Where the next partial pricing starts among the columns.
    next_price: usize,
    /// The last basis given to [`Lp::reset`], returned to when a refactorization finds the basis singular.
    fallback: Vec<usize>,
    degenerate_run: u32,
    stats: LpStats,
    /// Scratch, kept to avoid allocating in the pivot loop: `u` and `v` of FTRAN (`v` per row, nonzero only on the
    /// rows listed in `touched`, which `marked` flags), the entering column's entries on the C-rows (C-position,
    /// value), a row vector `g` over the C-positions, and the matrix of a refactorization.
    u: Vec<f64>,
    v: Vec<f64>,
    touched: Vec<u32>,
    marked: Vec<bool>,
    entering_c: Vec<(u32, f64)>,
    g: Vec<f64>,
    work: Vec<f64>,
}

impl Default for Lp {
    fn default() -> Self {
        Self::new()
    }
}

impl Lp {
    /// An LP with no rows and no columns.
    pub fn new() -> Self {
        Lp {
            kind: Vec::new(),
            rhs: Vec::new(),
            cpos: Vec::new(),
            slack: Vec::new(),
            y: Vec::new(),
            cost: Vec::new(),
            cols: Vec::new(),
            colpos: Vec::new(),
            dead: Vec::new(),
            crow: Vec::new(),
            scol: Vec::new(),
            xb: Vec::new(),
            binv: Vec::new(),
            stride: 0,
            valid: true,
            since_refactor: 0,
            next_price: 0,
            fallback: Vec::new(),
            degenerate_run: 0,
            stats: LpStats::default(),
            u: Vec::new(),
            v: Vec::new(),
            touched: Vec::new(),
            marked: Vec::new(),
            entering_c: Vec::new(),
            g: Vec::new(),
            work: Vec::new(),
        }
    }

    /// The number of rows.
    pub fn rows(&self) -> usize {
        self.kind.len()
    }

    /// The number of structural columns.
    pub fn columns(&self) -> usize {
        self.cols.len()
    }

    /// Adds a row. `entries` are the coefficients of existing columns in it (column id, value). A `Le` row joins with
    /// its slack basic (an S-row) and B_s unchanged; its slack value is computed (rhs − Σ over the structural basic
    /// columns), and must be ≥ −1e-9 (panic otherwise: the caller adds only rows the current point satisfies).
    /// An `Eq` row may be added only while no column exists yet, or must be followed by `reset` before `solve`.
    pub fn add_row(&mut self, kind: RowKind, rhs: f64, entries: &[(usize, f64)]) -> usize {
        let r = self.kind.len();
        assert!(r < NONE as usize, "Lp::add_row: too many rows");
        let mut slack = rhs;
        for &(j, a) in entries {
            assert!(j < self.cols.len(), "Lp::add_row: column {j} does not exist");
            let col = &mut self.cols[j];
            assert!(col.last().is_none_or(|e| e.0 != r as u32), "Lp::add_row: column {j} listed twice");
            col.push((r as u32, a));
            if self.colpos[j] != NONE {
                slack -= a * self.xb[self.colpos[j] as usize];
            }
        }
        self.kind.push(kind);
        self.rhs.push(rhs);
        self.cpos.push(NONE);
        self.y.push(0.0);
        self.v.push(0.0);
        self.marked.push(false);
        match kind {
            RowKind::Eq => {
                self.valid = false;
                self.slack.push(0.0);
            }
            RowKind::Le => {
                assert!(
                    !self.valid || slack >= -START_TOL,
                    "Lp::add_row: the current point violates row {r} by {:e}",
                    -slack
                );
                self.slack.push(slack);
            }
        }
        r
    }

    /// Adds a `Le` row that the current point may violate, keeping the basis: if the point violates it by `v`, an
    /// artificial column (entry −1 in the row, cost −`penalty`) enters in place of the row's slack at the value `v`
    /// (a pivot of the bordering kind, `B⁻¹a` being `−e_r`), so the basis stays primal feasible, and the primal
    /// simplex drives the artificial out, provided the LP is feasible and `penalty` exceeds the row's dual at its
    /// optima.  Returns the row and the artificial column, if one was needed.
    pub fn add_row_elastic(&mut self, rhs: f64, entries: &[(usize, f64)], penalty: f64) -> (usize, Option<usize>) {
        let valid = self.valid;
        self.valid = false;
        let r = self.add_row(RowKind::Le, rhs, entries);
        self.valid = valid;
        if !valid || self.slack[r] >= -START_TOL {
            return (r, None);
        }
        let t = self.add_column(-penalty, &[(r, -1.0)]);
        self.ftran(Enter::Column(t));
        let theta = self.slack[r] / self.v[r];
        self.pivot(Enter::Column(t), Leave::Slack(r), theta);
        (r, Some(t))
    }

    /// Adds a structural column, nonbasic at 0. Entries (row id, value), any order, no duplicates.
    pub fn add_column(&mut self, cost: f64, entries: &[(usize, f64)]) -> usize {
        let j = self.cols.len();
        assert!(j < NONE as usize, "Lp::add_column: too many columns");
        let mut col: Vec<(u32, f64)> = entries
            .iter()
            .map(|&(r, a)| {
                assert!(r < self.kind.len(), "Lp::add_column: row {r} does not exist");
                (r as u32, a)
            })
            .collect();
        col.sort_unstable_by_key(|e| e.0);
        assert!(col.windows(2).all(|w| w[0].0 < w[1].0), "Lp::add_column: a row listed twice");
        self.cols.push(col);
        self.cost.push(cost);
        self.colpos.push(NONE);
        self.dead.push(false);
        j
    }

    /// Sets the basis to the structural columns `basis` (as many as there are Eq rows, the C-rows then being exactly
    /// the Eq rows) and the slacks of all Le rows; refactorizes. Panics if singular or infeasible (beyond 1e-9).
    pub fn reset(&mut self, basis: &[usize]) {
        self.fallback = basis.to_vec();
        for &j in &self.scol {
            self.colpos[j as usize] = NONE;
        }
        self.scol.clear();
        self.xb.clear();
        self.crow.clear();
        self.cpos.fill(NONE);
        for (r, &kind) in self.kind.iter().enumerate() {
            if kind == RowKind::Eq {
                self.cpos[r] = self.crow.len() as u32;
                self.crow.push(r as u32);
            }
        }
        assert_eq!(basis.len(), self.crow.len(), "Lp::reset: the basis needs one column per Eq row");
        for &j in basis {
            assert!(j < self.cols.len(), "Lp::reset: column {j} does not exist");
            assert!(!self.dead[j], "Lp::reset: column {j} is dead");
            assert_eq!(self.colpos[j], NONE, "Lp::reset: column {j} listed twice");
            self.colpos[j] = self.scol.len() as u32;
            self.scol.push(j as u32);
            self.xb.push(0.0);
        }
        self.valid = true;
        self.degenerate_run = 0;
        self.stats.max_s = self.stats.max_s.max(self.scol.len());
        self.refactor();
        if let Some(p) = self.xb.iter().position(|&x| x < -START_TOL) {
            panic!("Lp::reset: column {} is at {:e}", self.scol[p], self.xb[p]);
        }
        if let Some(r) = (0..self.rows()).find(|&r| self.is_s_row(r) && self.slack[r] < -START_TOL) {
            panic!("Lp::reset: row {r} is violated by {:e}", -self.slack[r]);
        }
    }

    /// Marks a nonbasic column dead: it never enters again (panic if basic).
    pub fn kill(&mut self, j: usize) {
        assert_eq!(self.colpos[j], NONE, "Lp::kill: column {j} is basic");
        self.dead[j] = true;
    }

    /// Whether column `j` is basic.
    pub fn is_basic(&self, j: usize) -> bool {
        self.colpos[j] != NONE
    }

    /// Whether column `j` is dead.
    pub fn is_dead(&self, j: usize) -> bool {
        self.dead[j]
    }

    /// Primal simplex from the current basis until optimal or `max_pivots` pivots: Dantzig's rule (the largest
    /// reduced cost above 1e-9 among live nonbasic structural columns and nonbasic slacks), Harris's two-pass ratio
    /// test (tolerance 1e-9; among the candidates within the relaxed bound, the largest |pivot|; pivots below 1e-11
    /// in magnitude are never chosen), Bland's rule (the lowest-index eligible entering variable — structural columns
    /// by id, then slacks by row id — and the lowest-index leaving variable among ties) after 50 consecutive
    /// degenerate pivots (θ below 1e-12) until a nondegenerate pivot, refactorization every 64 pivots.
    ///
    /// Under Bland's rule the ties are the candidates within Harris's relaxed bound, and the index of a basic
    /// variable is its column id, or the number of columns plus the row id for a slack.  Panics if the LP is
    /// unbounded, or if an `Eq` row was added since the last [`Lp::reset`].
    pub fn solve(&mut self, max_pivots: u64) -> Status {
        assert!(self.valid, "Lp::solve: an Eq row was added since the last reset");
        let mut pivots = 0;
        loop {
            let bland = self.degenerate_run >= BLAND_AFTER;
            let Some(enter) = self.price(bland) else {
                return Status::Optimal;
            };
            if pivots == max_pivots {
                return Status::PivotLimit;
            }
            self.ftran(enter);
            let Some((leave, theta)) = self.ratio(bland) else {
                panic!("Lp::solve: the LP is unbounded");
            };
            self.pivot(enter, leave, theta);
            pivots += 1;
            self.stats.pivots += 1;
            self.stats.bland += bland as u64;
            if theta < DEGENERATE {
                self.stats.degenerate += 1;
                self.degenerate_run += 1;
            } else {
                self.degenerate_run = 0;
            }
            self.stats.max_s = self.stats.max_s.max(self.scol.len());
            self.since_refactor += 1;
            if self.since_refactor >= REFACTOR_EVERY && !self.factor() {
                self.stats.repairs += 1;
                let basis = std::mem::take(&mut self.fallback);
                self.reset(&basis);
            }
        }
    }

    /// The objective value c·x at the current basis.
    pub fn value(&self) -> f64 {
        self.scol.iter().zip(&self.xb).map(|(&j, &x)| self.cost[j as usize] * x).sum()
    }

    /// The duals y (one per row; 0 on S-rows), valid after `solve` / `reset`.
    pub fn duals(&self) -> &[f64] {
        &self.y
    }

    /// The value of structural column j (0 if nonbasic).
    pub fn primal(&self, j: usize) -> f64 {
        match self.colpos[j] {
            NONE => 0.0,
            p => self.xb[p as usize],
        }
    }

    /// The reduced cost at the current duals of a column not (yet) in the LP: cost − Σ y_r a_r.
    pub fn reduced_cost(&self, cost: f64, entries: &[(usize, f64)]) -> f64 {
        cost - entries.iter().map(|&(r, a)| self.y[r] * a).sum::<f64>()
    }

    /// Whether the current basis is primal feasible (all basic values ≥ −1e-7) — for tests and assertions.
    pub fn feasible(&self) -> bool {
        self.xb.iter().all(|&x| x >= -FEASIBLE_TOL)
            && (0..self.rows()).all(|r| !self.is_s_row(r) || self.slack[r] >= -FEASIBLE_TOL)
    }

    /// How far the current basis is from primal feasible: the largest of 0, the basic values' negatives and the S-rows'
    /// slacks' negatives (infinite if one is NaN).
    pub fn violation(&self) -> f64 {
        let slacks = (0..self.rows()).filter(|&r| self.is_s_row(r)).map(|r| self.slack[r]);
        self.xb
            .iter()
            .copied()
            .chain(slacks)
            .fold(0.0, |worst, x| if x.is_nan() { f64::INFINITY } else { worst.max(-x) })
    }

    /// The largest magnitude of a dual at the current basis (infinite if one is NaN).
    pub fn largest_dual(&self) -> f64 {
        self.y.iter().fold(0.0, |m, &y| if y.is_nan() { f64::INFINITY } else { m.max(y.abs()) })
    }

    /// The slack of row `r` at the current basis: its value for an S-row, 0 for a C-row (its slack nonbasic) and for
    /// an `Eq` row.
    pub fn slack(&self, r: usize) -> f64 {
        self.slack[r]
    }

    /// For the tests of a caller's handling of a breakdown: makes the current basis what rounding drift in `Binv` can
    /// leave, optimal by the duals but not primal feasible, by setting the first structural basic value to −1.
    #[cfg(test)]
    pub fn corrupt(&mut self) {
        self.xb[0] = -1.0;
    }

    /// The counters.
    pub fn stats(&self) -> LpStats {
        self.stats
    }

    /// Rebuilds `Binv` from `A` (`B_s` inverted by Gauss–Jordan elimination with partial pivoting) and recomputes
    /// the basic values and the duals from scratch.  Panics if the basis is singular.
    pub fn refactor(&mut self) {
        assert!(self.factor(), "Lp::refactor: the basis is singular");
    }

    /// [`refactor`](Self::refactor), returning false (and leaving the basis to be replaced) if it is singular.
    fn factor(&mut self) -> bool {
        let s = self.scol.len();
        self.reserve(s, 0);
        let st = self.stride;
        // B_s, row-major by C-position with stride s.
        let w = &mut self.work;
        w.clear();
        w.resize(s * s, 0.0);
        for (p, &j) in self.scol.iter().enumerate() {
            for &(r, a) in &self.cols[j as usize] {
                let q = self.cpos[r as usize];
                if q != NONE {
                    w[q as usize * s + p] = a;
                }
            }
        }
        // Row operations take [B_s | I] to [I | E]; then E B_s = I, so E = B_s⁻¹, its rows indexed by B_s's columns
        // (structural positions) and its columns by B_s's rows (C-positions).
        for p in 0..s {
            let row = &mut self.binv[p * st..p * st + s];
            row.fill(0.0);
            row[p] = 1.0;
        }
        for k in 0..s {
            let best = (k..s).max_by(|&a, &b| w[a * s + k].abs().total_cmp(&w[b * s + k].abs())).expect("k < s");
            let pivot = w[best * s + k];
            if pivot.abs() < SINGULAR {
                return false;
            }
            if best != k {
                swap_rows(w, s, s, best, k);
                swap_rows(&mut self.binv, st, s, best, k);
            }
            for x in &mut w[k * s + k..k * s + s] {
                *x /= pivot;
            }
            for x in &mut self.binv[k * st..k * st + s] {
                *x /= pivot;
            }
            for i in 0..s {
                let f = w[i * s + k];
                if i != k && f != 0.0 {
                    let (row, by) = two_rows(w, s, s, i, k);
                    add_scaled(&mut row[k..], -f, &by[k..]);
                    let (row, by) = two_rows(&mut self.binv, st, s, i, k);
                    add_scaled(row, -f, by);
                }
            }
        }
        // x_struct = Binv b_C.
        for p in 0..s {
            let row = &self.binv[p * st..p * st + s];
            self.xb[p] = row.iter().zip(&self.crow).map(|(&b, &r)| b * self.rhs[r as usize]).sum();
        }
        // The slack of S-row r: b_r − Σ_p A[r][scol[p]] x_struct[p].
        for r in 0..self.rows() {
            self.slack[r] = if self.is_s_row(r) { self.rhs[r] } else { 0.0 };
        }
        for (&j, &x) in self.scol.iter().zip(&self.xb) {
            for &(r, a) in &self.cols[j as usize] {
                if self.cpos[r as usize] == NONE {
                    self.slack[r as usize] -= a * x;
                }
            }
        }
        self.y.fill(0.0);
        self.compute_duals();
        self.since_refactor = 0;
        self.stats.refactors += 1;
        true
    }

    /// Whether row `r` is an S-row: a `Le` row whose slack is basic.
    fn is_s_row(&self, r: usize) -> bool {
        self.kind[r] == RowKind::Le && self.cpos[r] == NONE
    }

    /// The entering variable, `None` at an optimum: by Dantzig's rule (partial: over the slacks and the first
    /// segments of columns that yield one) the largest reduced cost above `PRICE_TOL`, by Bland's the eligible variable
    /// of lowest index, structural columns by id before slacks by row id.
    fn price(&mut self, bland: bool) -> Option<Enter> {
        let d_of = |lp: &Lp, j: usize| lp.cost[j] - lp.cols[j].iter().map(|&(r, a)| lp.y[r as usize] * a).sum::<f64>();
        let live = |lp: &Lp, j: usize| lp.colpos[j] == NONE && !lp.dead[j];
        if bland {
            if let Some(j) = (0..self.cols.len()).find(|&j| live(self, j) && d_of(self, j) > PRICE_TOL) {
                return Some(Enter::Column(j));
            }
            let slacks = self.crow.iter().map(|&r| r as usize).filter(|&r| self.kind[r] == RowKind::Le);
            return slacks.filter(|&r| -self.y[r] > PRICE_TOL).min().map(Enter::Slack);
        }
        let mut best = None;
        let mut best_d = PRICE_TOL;
        for &r in &self.crow {
            let r = r as usize;
            if self.kind[r] == RowKind::Le && -self.y[r] > best_d {
                (best, best_d) = (Some(Enter::Slack(r)), -self.y[r]);
            }
        }
        // Partial pricing: the columns in segments from where the last pricing stopped, until a segment yields a
        // candidate (a full cycle without one proves the basis optimal).
        let n = self.cols.len();
        let segment = (n / 8).max(PARTIAL);
        let (mut j, mut scanned) = (self.next_price.min(n.saturating_sub(1)), 0);
        while scanned < n {
            let end = (scanned + segment).min(n);
            while scanned < end {
                if live(self, j) {
                    let d = d_of(self, j);
                    if d > best_d {
                        (best, best_d) = (Some(Enter::Column(j)), d);
                    }
                }
                j = if j + 1 == n { 0 } else { j + 1 };
                scanned += 1;
            }
            if best.is_some() {
                break;
            }
        }
        self.next_price = j;
        best
    }

    /// FTRAN: `B⁻¹ a = (u, v)` for the entering variable's column `a`, with `u = Binv a_C` and, on the S-rows that
    /// `a` or the structural basic columns touch (listed in `touched`), `v_r = a_r − Σ_p A[r][scol[p]] u_p`.  The
    /// slack of C-row `r0` has `a = e_{r0}`: `u` is column `cpos[r0]` of `Binv`, and `a` is 0 on the S-rows.
    fn ftran(&mut self, enter: Enter) {
        let (s, st) = (self.scol.len(), self.stride);
        self.u.clear();
        match enter {
            Enter::Column(j) => {
                self.entering_c.clear();
                for &(r, a) in &self.cols[j] {
                    let q = self.cpos[r as usize];
                    if q != NONE {
                        self.entering_c.push((q, a));
                    } else {
                        touch(&mut self.marked, &mut self.touched, r);
                        self.v[r as usize] += a;
                    }
                }
                for p in 0..s {
                    let row = &self.binv[p * st..p * st + s];
                    self.u.push(self.entering_c.iter().map(|&(q, a)| row[q as usize] * a).sum());
                }
            }
            Enter::Slack(r0) => {
                let q0 = self.cpos[r0] as usize;
                self.u.extend((0..s).map(|p| self.binv[p * st + q0]));
            }
        }
        for (&j, &up) in self.scol.iter().zip(&self.u) {
            if up != 0.0 {
                for &(r, a) in &self.cols[j as usize] {
                    if self.cpos[r as usize] == NONE {
                        touch(&mut self.marked, &mut self.touched, r);
                        self.v[r as usize] -= a * up;
                    }
                }
            }
        }
    }

    /// Harris's two-pass ratio test over FTRAN's `(u, v)`: the leaving variable and the step `θ`, or `None` if
    /// nothing blocks (unbounded).  The first pass finds the relaxed bound `min (x_i + FEAS_TOL) / α_i` over the
    /// candidates (`α_i > PIVOT_TOL`); the second picks, among the candidates with `x_i / α_i` within it, the largest
    /// `α_i` (Bland's rule: the lowest index), and the step is its ratio, at least 0.
    fn ratio(&self, bland: bool) -> Option<(Leave, f64)> {
        let mut bound = f64::INFINITY;
        for (&x, &a) in self.xb.iter().zip(&self.u) {
            if a > PIVOT_TOL {
                bound = bound.min((x + FEAS_TOL) / a);
            }
        }
        for &r in &self.touched {
            let a = self.v[r as usize];
            if a > PIVOT_TOL {
                bound = bound.min((self.slack[r as usize] + FEAS_TOL) / a);
            }
        }
        if bound == f64::INFINITY {
            return None;
        }
        let ncols = self.cols.len();
        let mut chosen: Option<(Leave, f64, f64)> = None;
        let mut chosen_index = usize::MAX;
        let mut consider = |leave: Leave, x: f64, a: f64, index: usize| {
            if a > PIVOT_TOL && x / a <= bound {
                let better = match chosen {
                    None => true,
                    Some((_, _, chosen_a)) => {
                        if bland {
                            index < chosen_index
                        } else {
                            a > chosen_a
                        }
                    }
                };
                if better {
                    chosen = Some((leave, x, a));
                    chosen_index = index;
                }
            }
        };
        for (p, (&x, &a)) in self.xb.iter().zip(&self.u).enumerate() {
            consider(Leave::Column(p), x, a, self.scol[p] as usize);
        }
        for &r in &self.touched {
            let r = r as usize;
            consider(Leave::Slack(r), self.slack[r], self.v[r], ncols + r);
        }
        chosen.map(|(leave, x, a)| (leave, (x / a).max(0.0)))
    }

    /// Makes the pivot: the basic values step by `θ` (`x_B ← x_B − θ B⁻¹ a`, the entering variable at `θ`), `Binv`
    /// and the positions follow the basis by one of the four updates, and the duals are recomputed.
    fn pivot(&mut self, enter: Enter, leave: Leave, theta: f64) {
        for (x, &a) in self.xb.iter_mut().zip(&self.u) {
            *x -= theta * a;
        }
        for &r in &self.touched {
            self.slack[r as usize] -= theta * self.v[r as usize];
        }
        match (enter, leave) {
            (Enter::Column(j), Leave::Column(p)) => self.replace_column(j, p, theta),
            (Enter::Column(j), Leave::Slack(r)) => self.border(j, r, theta),
            (Enter::Slack(r0), Leave::Column(p)) => self.shrink(r0, p, theta),
            (Enter::Slack(r0), Leave::Slack(r)) => self.replace_row(r0, r, theta),
        }
        for &r in &self.touched {
            self.v[r as usize] = 0.0;
            self.marked[r as usize] = false;
        }
        self.touched.clear();
        self.compute_duals();
    }

    /// Case 1, structural column `j` replaces the one at position `p`.  `B_s' = B_s + (a_C − B_s e_p) e_pᵀ`, so
    /// `Binv B_s' = I + (u − e_p) e_pᵀ`, the identity with column `p` replaced by `u`, whose inverse is
    /// `I − (u − e_p) e_pᵀ / u_p`.  So `Binv'` is that inverse times `Binv`: row `p` divided by `u_p`, and every
    /// other row `i` minus `u_i` times the new row `p`.
    fn replace_column(&mut self, j: usize, p: usize, theta: f64) {
        let (s, st) = (self.scol.len(), self.stride);
        let up = self.u[p];
        for x in &mut self.binv[p * st..p * st + s] {
            *x /= up;
        }
        for (i, &f) in self.u.iter().enumerate() {
            if i != p && f != 0.0 {
                let (row, by) = two_rows(&mut self.binv, st, s, i, p);
                add_scaled(row, -f, by);
            }
        }
        self.colpos[self.scol[p] as usize] = NONE;
        self.scol[p] = j as u32;
        self.colpos[j] = p as u32;
        self.xb[p] = theta;
    }

    /// Case 2, structural column `j` enters and the slack of S-row `r` leaves: `r` becomes C-row `s` and `j`
    /// structural position `s`, so `B_s` grows by a border, `M = [[B_s, c], [dᵀ, δ]]` with `c = a_C`, `dᵀ` row `r`'s
    /// entries on the structural basic columns and `δ = a_r`.  With `u = Binv c`, `gᵀ = dᵀ Binv` and the Schur
    /// complement `σ = δ − dᵀ u`, `M⁻¹ = [[Binv + u gᵀ/σ, −u/σ], [−gᵀ/σ, 1/σ]]`: multiplying out with `B_s u = c`
    /// and `dᵀ Binv B_s = dᵀ` gives the identity.  And `σ = a_r − Σ_p A[r][scol[p]] u_p = v_r`, the pivot.
    fn border(&mut self, j: usize, r: usize, theta: f64) {
        let s = self.scol.len();
        self.reserve(s + 1, s);
        self.row_times_binv(r);
        let st = self.stride;
        let sigma = self.v[r];
        for (p, &up) in self.u.iter().enumerate() {
            let f = up / sigma;
            let row = &mut self.binv[p * st..p * st + s + 1];
            add_scaled(&mut row[..s], f, &self.g);
            row[s] = -f;
        }
        let row = &mut self.binv[s * st..s * st + s + 1];
        for (x, &g) in row.iter_mut().zip(&self.g) {
            *x = -g / sigma;
        }
        row[s] = 1.0 / sigma;
        self.colpos[j] = s as u32;
        self.scol.push(j as u32);
        self.xb.push(theta);
        self.cpos[r] = s as u32;
        self.crow.push(r as u32);
        self.slack[r] = 0.0;
    }

    /// Case 3, the slack of C-row `r0` (C-position `q0`) enters and the structural column at position `p` leaves:
    /// `r0` becomes an S-row and `B_s` loses row `q0` and column `p`.  Order `B_s` with them last,
    /// `[[M11, m12], [m21ᵀ, m22]]`, and `Binv` alike, `[[W11, w12], [w21ᵀ, h]]` (`W11` is `Binv` without row `p` and
    /// column `q0`, `w12` the rest of column `q0`, `w21ᵀ` the rest of row `p`, `h = Binv[p][q0]`).  Since `Binv`
    /// inverts `B_s`, `M11⁻¹` is the Schur complement of `h` in `Binv`, `W11 − w12 w21ᵀ / h`.  The entering column
    /// is `e_{r0}`, so `u` is column `q0` of `Binv` (`w12` is `u` without `u_p`) and the pivot `u_p` is `h`.  Row
    /// `p` and column `q0` are then removed by moving the last row and column into their places.
    fn shrink(&mut self, r0: usize, p: usize, theta: f64) {
        let (s, st) = (self.scol.len(), self.stride);
        let q0 = self.cpos[r0] as usize;
        let h = self.u[p];
        for x in &mut self.binv[p * st..p * st + s] {
            *x /= h;
        }
        for (i, &f) in self.u.iter().enumerate() {
            if i != p && f != 0.0 {
                let (row, by) = two_rows(&mut self.binv, st, s, i, p);
                add_scaled(row, -f, by);
            }
        }
        let last = s - 1;
        if p != last {
            self.binv.copy_within(last * st..last * st + s, p * st);
        }
        if q0 != last {
            for i in 0..last {
                self.binv[i * st + q0] = self.binv[i * st + last];
            }
        }
        self.colpos[self.scol[p] as usize] = NONE;
        self.scol.swap_remove(p);
        self.xb.swap_remove(p);
        if p != last {
            self.colpos[self.scol[p] as usize] = p as u32;
        }
        self.crow.swap_remove(q0);
        if q0 != last {
            self.cpos[self.crow[q0] as usize] = q0 as u32;
        }
        self.cpos[r0] = NONE;
        self.slack[r0] = theta;
        self.y[r0] = 0.0;
    }

    /// Case 4, the slack of C-row `r0` (C-position `q0`) enters and the slack of S-row `r` leaves: `r` takes
    /// C-position `q0` and `r0` becomes an S-row, so row `q0` of `B_s` becomes `dᵀ`, row `r`'s entries on the
    /// structural basic columns: `B_s' = B_s + e_{q0} zᵀ` with `zᵀ = dᵀ − e_{q0}ᵀ B_s`.  Sherman–Morrison gives
    /// `Binv' = Binv − Binv e_{q0} zᵀ Binv / (1 + zᵀ Binv e_{q0})`, where `Binv e_{q0} = u` (the entering column is
    /// `e_{r0}`), `zᵀ Binv = gᵀ − e_{q0}ᵀ` with `gᵀ = dᵀ Binv`, and `1 + zᵀ Binv e_{q0} = g_{q0}`.  So
    /// `Binv' = Binv − u (gᵀ − e_{q0}ᵀ) / g_{q0}`, and `g_{q0} = dᵀ u = −v_r` is nonzero, `v_r` being the pivot.
    fn replace_row(&mut self, r0: usize, r: usize, theta: f64) {
        let (s, st) = (self.scol.len(), self.stride);
        let q0 = self.cpos[r0] as usize;
        self.row_times_binv(r);
        let gq0 = self.g[q0];
        self.g[q0] -= 1.0;
        for x in &mut self.g {
            *x /= gq0;
        }
        for (p, &f) in self.u.iter().enumerate() {
            if f != 0.0 {
                add_scaled(&mut self.binv[p * st..p * st + s], -f, &self.g);
            }
        }
        self.crow[q0] = r as u32;
        self.cpos[r] = q0 as u32;
        self.cpos[r0] = NONE;
        self.slack[r0] = theta;
        self.slack[r] = 0.0;
        self.y[r0] = 0.0;
    }

    /// `g = dᵀ Binv` (over the C-positions), with `d_p = A[r][scol[p]]` found by binary search in the sorted
    /// columns.
    fn row_times_binv(&mut self, r: usize) {
        let (s, st) = (self.scol.len(), self.stride);
        self.g.clear();
        self.g.resize(s, 0.0);
        for (p, &j) in self.scol.iter().enumerate() {
            let col = &self.cols[j as usize];
            if let Ok(i) = col.binary_search_by_key(&(r as u32), |e| e.0) {
                add_scaled(&mut self.g, col[i].1, &self.binv[p * st..p * st + s]);
            }
        }
    }

    /// `y_C = c_struct Binv`, put on the C-rows (the S-rows' duals stay 0).
    fn compute_duals(&mut self) {
        let (s, st) = (self.scol.len(), self.stride);
        self.g.clear();
        self.g.resize(s, 0.0);
        for (p, &j) in self.scol.iter().enumerate() {
            let c = self.cost[j as usize];
            if c != 0.0 {
                add_scaled(&mut self.g, c, &self.binv[p * st..p * st + s]);
            }
        }
        for (&r, &y) in self.crow.iter().zip(&self.g) {
            self.y[r as usize] = y;
        }
    }

    /// Makes room in `binv` for `n` rows and columns, keeping the first `keep` of each.
    fn reserve(&mut self, n: usize, keep: usize) {
        if n <= self.stride {
            return;
        }
        let stride = n.max(2 * self.stride).max(16);
        let mut binv = vec![0.0; stride * stride];
        for p in 0..keep {
            binv[p * stride..p * stride + keep].copy_from_slice(&self.binv[p * self.stride..p * self.stride + keep]);
        }
        self.binv = binv;
        self.stride = stride;
    }
}

/// Lists row `r` in `touched` unless `marked` says it is there.
fn touch(marked: &mut [bool], touched: &mut Vec<u32>, r: u32) {
    if !marked[r as usize] {
        marked[r as usize] = true;
        touched.push(r);
    }
}

/// `row += f · by`.
fn add_scaled(row: &mut [f64], f: f64, by: &[f64]) {
    for (x, &b) in row.iter_mut().zip(by) {
        *x += f * b;
    }
}

/// Rows `i` (to change) and `k` (`≠ i`) of a row-major matrix, their first `len` entries.
fn two_rows(m: &mut [f64], stride: usize, len: usize, i: usize, k: usize) -> (&mut [f64], &[f64]) {
    if i < k {
        let (lo, hi) = m.split_at_mut(k * stride);
        (&mut lo[i * stride..i * stride + len], &hi[..len])
    } else {
        let (lo, hi) = m.split_at_mut(i * stride);
        (&mut hi[..len], &lo[k * stride..k * stride + len])
    }
}

/// Swaps the first `len` entries of rows `i` and `k` (`≠ i`) of a row-major matrix.
fn swap_rows(m: &mut [f64], stride: usize, len: usize, i: usize, k: usize) {
    let (a, b) = (i.min(k), i.max(k));
    let (lo, hi) = m.split_at_mut(b * stride);
    lo[a * stride..a * stride + len].swap_with_slice(&mut hi[..len]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;
    use std::time::Instant;

    /// xorshift64*.
    struct Rng(u64);

    impl Rng {
        fn new(seed: u64) -> Self {
            Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0x2545_f491_4f6c_dd1d)
        }

        fn bits(&mut self) -> u64 {
            self.0 ^= self.0 >> 12;
            self.0 ^= self.0 << 25;
            self.0 ^= self.0 >> 27;
            self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
        }

        fn below(&mut self, n: usize) -> usize {
            (self.bits() % n as u64) as usize
        }

        /// Uniform in [0, 1).
        fn unit(&mut self) -> f64 {
            (self.bits() >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    /// An LP as the tests build it: the `Lp`, and its data for the reference and the checks.
    struct Model {
        lp: Lp,
        kind: Vec<RowKind>,
        rhs: Vec<f64>,
        cost: Vec<f64>,
        /// Per column, its entries (row, value).
        cols: Vec<Vec<(usize, f64)>>,
        dead: Vec<bool>,
    }

    impl Model {
        fn new() -> Self {
            Model {
                lp: Lp::new(),
                kind: Vec::new(),
                rhs: Vec::new(),
                cost: Vec::new(),
                cols: Vec::new(),
                dead: Vec::new(),
            }
        }

        fn row(&mut self, kind: RowKind, rhs: f64, entries: &[(usize, f64)]) -> usize {
            let r = self.lp.add_row(kind, rhs, entries);
            assert_eq!(r, self.kind.len());
            for &(j, a) in entries {
                self.cols[j].push((r, a));
            }
            self.kind.push(kind);
            self.rhs.push(rhs);
            r
        }

        fn column(&mut self, cost: f64, entries: &[(usize, f64)]) -> usize {
            let j = self.lp.add_column(cost, entries);
            assert_eq!(j, self.cost.len());
            self.cost.push(cost);
            self.cols.push(entries.to_vec());
            self.dead.push(false);
            j
        }

        fn kill(&mut self, j: usize) {
            self.lp.kill(j);
            self.dead[j] = true;
        }

        fn rows_of(&self, kind: RowKind) -> Vec<usize> {
            (0..self.kind.len()).filter(|&r| self.kind[r] == kind).collect()
        }
    }

    /// A `Le` row's rhs: tiny (like the study's perturbations) or 0, 1 or 2.
    fn random_rhs(rng: &mut Rng) -> f64 {
        if rng.below(2) == 0 {
            1e-7 * rng.unit()
        } else {
            rng.below(3) as f64
        }
    }

    /// A cost in [0, 10], an integer half the time (for ties and degeneracy).
    fn random_cost(rng: &mut Rng) -> f64 {
        if rng.below(2) == 0 {
            rng.below(11) as f64
        } else {
            10.0 * rng.unit()
        }
    }

    /// A column of the master's shape: entry 1 in a random `Eq` row, −1, 0 or 1 on each `Le` row (1 on `plus`).
    fn random_column(rng: &mut Rng, m: &mut Model, plus: Option<usize>) -> usize {
        let eqs = m.rows_of(RowKind::Eq);
        let mut entries = vec![(eqs[rng.below(eqs.len())], 1.0)];
        for r in m.rows_of(RowKind::Le) {
            let a = if plus == Some(r) { 1.0 } else { rng.below(3) as f64 - 1.0 };
            if a != 0.0 {
                entries.push((r, a));
            }
        }
        let cost = random_cost(rng);
        m.column(cost, &entries)
    }

    /// A random LP of the master's shape: 1-3 `Eq` rows of rhs 1, each with an empty column (cost 0, entry 1 in
    /// that row only), 0-5 `Le` rows and 1-6 further columns.  Returns it with its empty columns, a feasible basis.
    fn random_model(rng: &mut Rng) -> (Model, Vec<usize>) {
        let mut m = Model::new();
        let eqs: Vec<usize> = (0..1 + rng.below(3)).map(|_| m.row(RowKind::Eq, 1.0, &[])).collect();
        for _ in 0..rng.below(6) {
            let rhs = random_rhs(rng);
            m.row(RowKind::Le, rhs, &[]);
        }
        let empties = eqs.iter().map(|&e| m.column(0.0, &[(e, 1.0)])).collect();
        for _ in 0..1 + rng.below(6) {
            random_column(rng, &mut m, None);
        }
        (m, empties)
    }

    /// The optimum by enumeration.  The live columns and the `Le` rows' slacks have full row rank (the empty
    /// columns and the slacks make an identity), and the LP is bounded and feasible, so its optimum is attained at
    /// a basic feasible solution: a set of as many columns as rows whose matrix is nonsingular, with a nonnegative
    /// solution.  The best objective over all such sets.
    fn reference(m: &Model) -> f64 {
        let rows = m.kind.len();
        let mut cols: Vec<(f64, Vec<f64>)> = Vec::new();
        for (j, entries) in m.cols.iter().enumerate() {
            if !m.dead[j] {
                let mut a = vec![0.0; rows];
                for &(r, v) in entries {
                    a[r] = v;
                }
                cols.push((m.cost[j], a));
            }
        }
        for r in m.rows_of(RowKind::Le) {
            let mut a = vec![0.0; rows];
            a[r] = 1.0;
            cols.push((0.0, a));
        }
        let n = cols.len();
        let mut best = f64::NEG_INFINITY;
        let mut pick: Vec<usize> = (0..rows).collect();
        let mut mat = vec![0.0; rows * (rows + 1)];
        loop {
            if let Some(value) = basic_value(&cols, &pick, &m.rhs, &mut mat) {
                best = best.max(value);
            }
            // The next set in lexicographic order.
            let Some(i) = (0..rows).rev().find(|&i| pick[i] < n - rows + i) else {
                return best;
            };
            pick[i] += 1;
            for k in i + 1..rows {
                pick[k] = pick[k - 1] + 1;
            }
        }
    }

    /// The objective at the basic solution of the columns `pick` (Gaussian elimination with partial pivoting), if
    /// their matrix is nonsingular and the solution nonnegative.
    fn basic_value(cols: &[(f64, Vec<f64>)], pick: &[usize], rhs: &[f64], mat: &mut [f64]) -> Option<f64> {
        let m = pick.len();
        let w = m + 1;
        for i in 0..m {
            for (k, &j) in pick.iter().enumerate() {
                mat[i * w + k] = cols[j].1[i];
            }
            mat[i * w + m] = rhs[i];
        }
        for k in 0..m {
            let best = (k..m).max_by(|&a, &b| mat[a * w + k].abs().total_cmp(&mat[b * w + k].abs()))?;
            if mat[best * w + k].abs() < 1e-9 {
                return None;
            }
            for c in 0..w {
                mat.swap(k * w + c, best * w + c);
            }
            for i in k + 1..m {
                let f = mat[i * w + k] / mat[k * w + k];
                for c in k..w {
                    mat[i * w + c] -= f * mat[k * w + c];
                }
            }
        }
        let mut x = vec![0.0; m];
        for k in (0..m).rev() {
            let t = mat[k * w + m] - (k + 1..m).map(|c| mat[k * w + c] * x[c]).sum::<f64>();
            x[k] = t / mat[k * w + k];
        }
        x.iter().all(|&v| v >= -1e-9).then(|| pick.iter().zip(&x).map(|(&j, &v)| cols[j].0 * v).sum())
    }

    /// Checks an optimum against the model: primal feasibility, the value, dual feasibility (the reduced costs of
    /// the live columns at most 1e-7, the `Le` rows' duals at least −1e-7) and `y·b` equal to the value.
    fn check_optimum(m: &Model) {
        let lp = &m.lp;
        assert!(lp.feasible(), "infeasible basis");
        let value = lp.value();
        let y = lp.duals();
        let mut ax = vec![0.0; m.kind.len()];
        let mut cx = 0.0;
        for (j, entries) in m.cols.iter().enumerate() {
            let x = lp.primal(j);
            assert!(x >= -1e-7, "column {j} at {x}");
            if m.dead[j] {
                assert_eq!(x, 0.0, "dead column {j} at {x}");
            } else {
                let d = lp.reduced_cost(m.cost[j], entries);
                assert!(d <= 1e-7, "column {j} has reduced cost {d}");
            }
            cx += m.cost[j] * x;
            for &(r, a) in entries {
                ax[r] += a * x;
            }
        }
        for (r, &kind) in m.kind.iter().enumerate() {
            match kind {
                RowKind::Eq => assert!((ax[r] - m.rhs[r]).abs() <= 1e-7, "row {r}: {} = {}", ax[r], m.rhs[r]),
                RowKind::Le => {
                    assert!(ax[r] <= m.rhs[r] + 1e-7, "row {r}: {} > {}", ax[r], m.rhs[r]);
                    assert!(y[r] >= -1e-7, "row {r} has dual {}", y[r]);
                }
            }
        }
        assert!((cx - value).abs() <= 1e-7, "c·x {cx}, value {value}");
        let yb: f64 = y.iter().zip(&m.rhs).map(|(y, b)| y * b).sum();
        assert!((yb - value).abs() <= 1e-6, "y·b {yb}, value {value}");
    }

    /// Checks the partitioned basis against its definition: the positions agree, `B_s Binv = I`, and the basic
    /// values and the duals are those recomputed from `A`, all within `tol`.
    fn check_invariants(lp: &Lp, tol: f64) {
        let (s, st, rows) = (lp.scol.len(), lp.stride, lp.rows());
        assert_eq!(lp.crow.len(), s);
        assert_eq!(lp.xb.len(), s);
        for (p, &j) in lp.scol.iter().enumerate() {
            assert_eq!(lp.colpos[j as usize], p as u32);
        }
        for (q, &r) in lp.crow.iter().enumerate() {
            assert_eq!(lp.cpos[r as usize], q as u32);
        }
        assert_eq!(lp.colpos.iter().filter(|&&p| p != NONE).count(), s);
        assert_eq!(lp.cpos.iter().filter(|&&q| q != NONE).count(), s);
        for r in 0..rows {
            assert!(lp.kind[r] == RowKind::Le || lp.cpos[r] != NONE, "Eq row {r} is not a C-row");
            assert!(lp.cpos[r] != NONE || lp.y[r] == 0.0, "S-row {r} has dual {}", lp.y[r]);
            assert!(!lp.marked[r] && lp.v[r] == 0.0, "scratch of row {r} not cleared");
        }
        assert!(lp.touched.is_empty());
        let mut bs = vec![0.0; s * s];
        for (p, &j) in lp.scol.iter().enumerate() {
            for &(r, a) in &lp.cols[j as usize] {
                if lp.cpos[r as usize] != NONE {
                    bs[lp.cpos[r as usize] as usize * s + p] = a;
                }
            }
        }
        for q in 0..s {
            for q2 in 0..s {
                let e: f64 = (0..s).map(|p| bs[q * s + p] * lp.binv[p * st + q2]).sum();
                let want = if q == q2 { 1.0 } else { 0.0 };
                assert!((e - want).abs() <= tol, "(B_s Binv)[{q}][{q2}] = {e}");
            }
        }
        for p in 0..s {
            let x: f64 = (0..s).map(|q| lp.binv[p * st + q] * lp.rhs[lp.crow[q] as usize]).sum();
            assert!((x - lp.xb[p]).abs() <= tol, "x_struct[{p}] = {}, recomputed {x}", lp.xb[p]);
        }
        let mut slack: Vec<f64> = (0..rows).map(|r| if lp.is_s_row(r) { lp.rhs[r] } else { 0.0 }).collect();
        for (&j, &x) in lp.scol.iter().zip(&lp.xb) {
            for &(r, a) in &lp.cols[j as usize] {
                slack[r as usize] -= if lp.cpos[r as usize] == NONE { a * x } else { 0.0 };
            }
        }
        for r in (0..rows).filter(|&r| lp.is_s_row(r)) {
            assert!(
                (slack[r] - lp.slack[r]).abs() <= tol,
                "slack of row {r} = {}, recomputed {}",
                lp.slack[r],
                slack[r]
            );
        }
        for q in 0..s {
            let y: f64 = (0..s).map(|p| lp.cost[lp.scol[p] as usize] * lp.binv[p * st + q]).sum();
            let r = lp.crow[q] as usize;
            assert!((y - lp.y[r]).abs() <= tol, "y[{r}] = {}, recomputed {y}", lp.y[r]);
        }
    }

    /// The basis as sets: the structural columns and the C-rows.
    fn basis_sets(lp: &Lp) -> (Vec<u32>, Vec<u32>) {
        let (mut cols, mut rows) = (lp.scol.clone(), lp.crow.clone());
        cols.sort_unstable();
        rows.sort_unstable();
        (cols, rows)
    }

    /// Which of the four pivot cases (0-3 for 1-4) took the basis `before` to `lp`'s.
    fn pivot_case(before: &(Vec<u32>, Vec<u32>), lp: &Lp) -> usize {
        let (cols, rows) = basis_sets(lp);
        match cols.len().cmp(&before.0.len()) {
            Ordering::Greater => 1,
            Ordering::Less => 2,
            Ordering::Equal if rows != before.1 => 3,
            Ordering::Equal => {
                assert_ne!(cols, before.0, "a pivot left the basis as it was");
                0
            }
        }
    }

    /// Solves to an optimum and compares with the reference.
    fn solve_and_compare(m: &mut Model) {
        assert_eq!(m.lp.solve(100_000), Status::Optimal);
        check_invariants(&m.lp, 1e-8);
        let want = reference(m);
        assert!((m.lp.value() - want).abs() <= 1e-6, "value {}, reference {want}", m.lp.value());
        check_optimum(m);
    }

    /// Test 1: random LPs of the master's shape, solved one pivot at a time with the basis checked after each,
    /// match the reference, and all four pivot cases occur.
    #[test]
    fn random_lps_match_the_reference() {
        let mut rng = Rng::new(1);
        let mut cases = [0u64; 4];
        for _ in 0..3000 {
            let (mut m, empties) = random_model(&mut rng);
            m.lp.reset(&empties);
            check_invariants(&m.lp, 1e-8);
            loop {
                let (before, pivots) = (basis_sets(&m.lp), m.lp.stats().pivots);
                let status = m.lp.solve(1);
                if m.lp.stats().pivots > pivots {
                    cases[pivot_case(&before, &m.lp)] += 1;
                    check_invariants(&m.lp, 1e-8);
                }
                if status == Status::Optimal {
                    break;
                }
                assert!(m.lp.stats().pivots < 1000, "too many pivots");
            }
            let want = reference(&m);
            assert!((m.lp.value() - want).abs() <= 1e-6, "value {}, reference {want}", m.lp.value());
            check_optimum(&m);
        }
        eprintln!("pivots of cases 1-4: {cases:?}");
        assert!(cases.iter().all(|&c| c > 0), "a pivot case never occurred: {cases:?}");
    }

    /// Test 2: LPs built in stages (columns added; a `Le` row that the current point satisfies, with entries −1 or
    /// 0 on the existing columns, and new columns with entry 1 in it) match the reference at each stage.
    #[test]
    fn incremental_lps_match_the_reference() {
        let mut rng = Rng::new(2);
        for _ in 0..1000 {
            let (mut m, empties) = random_model(&mut rng);
            m.lp.reset(&empties);
            solve_and_compare(&mut m);
            for _ in 0..1 + rng.below(3) {
                random_column(&mut rng, &mut m, None);
            }
            solve_and_compare(&mut m);
            let entries: Vec<(usize, f64)> =
                (0..m.cost.len()).filter(|_| rng.below(2) == 0).map(|j| (j, -1.0)).collect();
            let rhs = random_rhs(&mut rng);
            let r = m.row(RowKind::Le, rhs, &entries);
            check_invariants(&m.lp, 1e-8);
            for _ in 0..1 + rng.below(3) {
                random_column(&mut rng, &mut m, Some(r));
            }
            solve_and_compare(&mut m);
        }
    }

    /// Test 3: after a reset to the empty columns and the death of a random subset of the others, the optimum is
    /// that of the LP without the dead columns.
    #[test]
    fn dead_columns_never_enter() {
        let mut rng = Rng::new(3);
        for _ in 0..1000 {
            let (mut m, empties) = random_model(&mut rng);
            m.lp.reset(&empties);
            solve_and_compare(&mut m);
            m.lp.reset(&empties);
            for j in 0..m.cost.len() {
                if !empties.contains(&j) && rng.below(2) == 0 {
                    m.kill(j);
                }
            }
            assert!((0..m.cost.len()).all(|j| m.lp.is_dead(j) == m.dead[j]));
            solve_and_compare(&mut m);
        }
    }

    #[test]
    #[should_panic(expected = "is basic")]
    fn killing_a_basic_column_panics() {
        let mut m = Model::new();
        m.row(RowKind::Eq, 1.0, &[]);
        let e = m.column(0.0, &[(0, 1.0)]);
        m.lp.reset(&[e]);
        m.kill(e);
    }

    #[test]
    #[should_panic(expected = "violates row")]
    fn adding_a_violated_row_panics() {
        let mut m = Model::new();
        m.row(RowKind::Eq, 1.0, &[]);
        let e = m.column(0.0, &[(0, 1.0)]);
        m.lp.reset(&[e]);
        m.row(RowKind::Le, 0.5, &[(e, 1.0)]);
    }

    /// Beale's example, on which Dantzig's rule with the textbook ties cycles: no `Eq` rows, so `s` starts at 0.  It
    /// starts with a degenerate pivot, so with a run of `BLAND_AFTER − 1` already made the next pivots are Bland's.
    #[test]
    fn beales_example() {
        for run in [0, BLAND_AFTER - 1] {
            let mut m = Model::new();
            for rhs in [0.0, 0.0, 1.0] {
                m.row(RowKind::Le, rhs, &[]);
            }
            m.column(0.75, &[(0, 0.25), (1, 0.5)]);
            m.column(-20.0, &[(0, -8.0), (1, -12.0)]);
            m.column(0.5, &[(0, -1.0), (1, -0.5), (2, 1.0)]);
            m.column(-6.0, &[(0, 9.0), (1, 3.0)]);
            m.lp.reset(&[]);
            m.lp.degenerate_run = run;
            solve_and_compare(&mut m);
            assert!((m.lp.value() - 1.25).abs() <= 1e-9);
            assert_eq!(m.lp.stats().bland > 0, run > 0, "{:?}", m.lp.stats());
        }
    }

    /// The data of an LP of test 4: `Eq` rows, `Le` rows of rhs 0, and the columns (cost, entries), the empty
    /// columns first.
    struct Degenerate {
        eqs: usize,
        les: usize,
        columns: Vec<(f64, Vec<(usize, f64)>)>,
    }

    impl Degenerate {
        fn random(rng: &mut Rng) -> Self {
            let (eqs, les) = (1 + rng.below(3), 10 + rng.below(11));
            let mut columns: Vec<_> = (0..eqs).map(|e| (0.0, vec![(e, 1.0)])).collect();
            for _ in 0..20 + rng.below(21) {
                let mut entries = vec![(rng.below(eqs), 1.0)];
                for r in eqs..eqs + les {
                    let a = rng.below(3) as f64 - 1.0;
                    if a != 0.0 {
                        entries.push((r, a));
                    }
                }
                columns.push((rng.below(11) as f64, entries));
            }
            Degenerate { eqs, les, columns }
        }

        /// The LP with column `order[i]` added `i`-th, reset to its empty columns.
        fn build(&self, order: &[usize]) -> Model {
            let mut m = Model::new();
            for _ in 0..self.eqs {
                m.row(RowKind::Eq, 1.0, &[]);
            }
            for _ in 0..self.les {
                m.row(RowKind::Le, 0.0, &[]);
            }
            let mut empties = vec![0; self.eqs];
            for &k in order {
                let j = m.column(self.columns[k].0, &self.columns[k].1);
                if k < self.eqs {
                    empties[k] = j;
                }
            }
            m.lp.reset(&empties);
            m
        }
    }

    /// Test 4: degenerate LPs (`Le` rows of rhs 0) reach the optimum, the same value in a permuted column order and
    /// by Bland's rule alone.
    #[test]
    fn degenerate_lps_reach_the_optimum() {
        let mut rng = Rng::new(4);
        let mut stats = LpStats::default();
        let mut bland_alone = 0;
        for _ in 0..200 {
            let data = Degenerate::random(&mut rng);
            let n = data.columns.len();
            let identity: Vec<usize> = (0..n).collect();
            let mut permuted = identity.clone();
            for i in (1..n).rev() {
                permuted.swap(i, rng.below(i + 1));
            }
            let mut values = Vec::new();
            for order in [&identity, &permuted] {
                let mut m = data.build(order);
                assert_eq!(m.lp.solve(100_000), Status::Optimal);
                check_invariants(&m.lp, 1e-8);
                check_optimum(&m);
                values.push(m.lp.value());
                let s = m.lp.stats();
                stats.pivots += s.pivots;
                stats.degenerate += s.degenerate;
                stats.bland += s.bland;
            }
            // Bland's rule alone, from the start.
            let mut m = data.build(&identity);
            loop {
                m.lp.degenerate_run = BLAND_AFTER;
                if m.lp.solve(1) == Status::Optimal {
                    break;
                }
                assert!(m.lp.stats().pivots < 100_000, "Bland's rule cycles");
            }
            assert_eq!(m.lp.stats().bland, m.lp.stats().pivots);
            bland_alone += m.lp.stats().pivots;
            check_invariants(&m.lp, 1e-8);
            check_optimum(&m);
            values.push(m.lp.value());
            assert!(values.iter().all(|v| (v - values[0]).abs() <= 1e-6), "values {values:?}");
        }
        eprintln!(
            "degenerate LPs: {} pivots, {} degenerate, {} by Bland's rule; {bland_alone} pivots by Bland's rule alone",
            stats.pivots, stats.degenerate, stats.bland
        );
    }

    /// An LP of the master's shape at a larger size, and its empty columns: 40 `Eq` rows, 1,500 `Le` rows of rhs 1 or
    /// 2 (a tenth tiny), 3,000 sparse columns (1 in an `Eq` row, 1-10 entries in `Le` rows, 1 or in [0.5, 1.5),
    /// positive three times in four).  Most `Le` rows stay slack, as in the master: with two thirds of the rows of rhs
    /// near 0 and 1-30 entries of either sign, 900 rows became tight and the LP took 100,000 pivots.
    fn large_model(seed: u64) -> (Model, Vec<usize>) {
        let mut rng = Rng::new(seed);
        let (eqs, les) = (40, 1500);
        let mut m = Model::new();
        for _ in 0..eqs {
            m.row(RowKind::Eq, 1.0, &[]);
        }
        for _ in 0..les {
            let rhs = if rng.below(10) == 0 { 1e-7 * rng.unit() } else { 1.0 + rng.below(2) as f64 };
            m.row(RowKind::Le, rhs, &[]);
        }
        let empties = (0..eqs).map(|e| m.column(0.0, &[(e, 1.0)])).collect();
        for _ in 0..3000 {
            let mut entries = vec![(rng.below(eqs), 1.0)];
            for _ in 0..1 + rng.below(10) {
                let r = eqs + rng.below(les);
                if entries.iter().all(|e| e.0 != r) {
                    let size = if rng.below(2) == 0 { 1.0 } else { 0.5 + rng.unit() };
                    entries.push((r, if rng.below(4) != 0 { size } else { -size }));
                }
            }
            let cost = 10.0 * rng.unit();
            m.column(cost, &entries);
        }
        (m, empties)
    }

    /// Test 5: larger LPs solved in one call and in chunks of 37 pivots reach the same value, both optimal, and a
    /// refactorization at the end moves the duals by at most 1e-6.
    #[test]
    fn large_lps_solve_alike_in_chunks() {
        for seed in 5..10 {
            let (mut whole, empties) = large_model(seed);
            whole.lp.reset(&empties);
            let start = Instant::now();
            assert_eq!(whole.lp.solve(1_000_000), Status::Optimal);
            let elapsed = start.elapsed();
            let (mut chunked, empties) = large_model(seed);
            chunked.lp.reset(&empties);
            let mut calls = 1;
            while chunked.lp.solve(37) != Status::Optimal {
                calls += 1;
                assert!(calls < 100_000, "too many calls");
            }
            for m in [&whole, &chunked] {
                check_optimum(m);
                check_invariants(&m.lp, 1e-7);
            }
            let (a, b) = (whole.lp.value(), chunked.lp.value());
            assert!((a - b).abs() <= 1e-6, "values {a} and {b}");
            let y = whole.lp.duals().to_vec();
            whole.lp.refactor();
            let moved = y.iter().zip(whole.lp.duals()).map(|(a, b)| (a - b).abs()).fold(0.0, f64::max);
            assert!(moved <= 1e-6, "a refactorization moved the duals by {moved:e}");
            eprintln!(
                "seed {seed}: value {a:.6}, {:?}, {calls} calls of 37, one call {elapsed:.2?}, duals moved {moved:.1e}",
                whole.lp.stats()
            );
        }
    }
}
