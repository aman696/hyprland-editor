//! Shortest insertion/deletion scripts with linear frontier space. The middle
//! partition and independent forbidden-edge search never use heuristic cleanup.
use super::{Budget, MAX_ITEMS};
use std::ops::Range;
#[derive(Clone, Debug)]
pub(super) struct Hunk {
    pub old: Range<usize>,
    pub new: Range<usize>,
    pub footprint: Range<usize>,
    pub uncertain: bool,
}
#[derive(Debug)]
pub(super) struct Script {
    pub hunks: Vec<Hunk>,
    pub equals: Vec<(Range<usize>, Range<usize>)>,
    pub distance: usize,
}
#[derive(Clone, Copy)]
enum Step {
    Equal(usize, usize, usize),
    Change(usize, usize, usize, usize),
}
fn append(steps: &mut Vec<Step>, step: Step) -> Result<(), String> {
    match (steps.last_mut(), step) {
        (Some(Step::Equal(a, b, n)), Step::Equal(x, y, m)) if *a + *n == x && *b + *n == y => {
            *n += m
        }
        (Some(Step::Change(a, b, c, d)), Step::Change(x, y, z, w)) if *b == x && *d == z => {
            *b = y;
            *d = w;
            let _ = (a, c);
        }
        _ => {
            if steps.len() >= MAX_ITEMS * 2 {
                return Err("Merge diff exceeds its operation budget".into());
            }
            steps.push(step);
        }
    }
    Ok(())
}
fn split(a: &[u32], b: &[u32], budget: &mut Budget) -> Result<Option<(usize, usize)>, String> {
    let n = a.len() as i32;
    let m = b.len() as i32;
    let max = (n + m + 1) / 2;
    let offset = max + 1;
    let size = (2 * max + 3) as usize;
    budget.frontier(size * 2)?;
    let (mut front, mut back) = (vec![-1_i32; size], vec![-1_i32; size]);
    front[(offset + 1) as usize] = 0;
    back[(offset + 1) as usize] = 0;
    let delta = n - m;
    for depth in 0..=max {
        for reverse in [false, true] {
            let (active, other) = if reverse {
                (&mut back, &front)
            } else {
                (&mut front, &back)
            };
            for diagonal in (-depth..=depth).step_by(2) {
                budget.work(1)?;
                let cell = (offset + diagonal) as usize;
                let insert = active[cell + 1];
                let delete = if active[cell - 1] >= 0 {
                    active[cell - 1] + 1
                } else {
                    -1
                };
                let mut x = insert.max(delete);
                let mut y = x - diagonal;
                if x < 0 || y < 0 || x > n || y > m {
                    active[cell] = -1;
                    continue;
                }
                while x < n && y < m {
                    budget.work(1)?;
                    let same = if reverse {
                        a[(n - x - 1) as usize] == b[(m - y - 1) as usize]
                    } else {
                        a[x as usize] == b[y as usize]
                    };
                    if !same {
                        break;
                    }
                    x += 1;
                    y += 1;
                }
                active[cell] = x;
                if (delta & 1 != 0) != reverse {
                    let opposite = offset + delta - diagonal;
                    if opposite >= 0
                        && (opposite as usize) < size
                        && other[opposite as usize] >= 0
                        && x + other[opposite as usize] >= n
                    {
                        let (at, bt) = if reverse {
                            let fx = other[opposite as usize];
                            (fx, fx - (delta - diagonal))
                        } else {
                            (x, y)
                        };
                        if (at > 0 || bt > 0) && (at < n || bt < m) {
                            return Ok(Some((at as usize, bt as usize)));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}
/// Does any shortest path remain after forbidding this exact equality edge?
/// A false result proves this anchor is mandatory for the chosen edit distance.
pub(super) fn alternative(
    a: &[u32],
    b: &[u32],
    edge: (usize, usize),
    distance: usize,
    budget: &mut Budget,
) -> Result<bool, String> {
    let offset = distance as i32 + 1;
    let size = 2 * distance + 3;
    budget.frontier(size)?;
    let mut frontier = vec![-1_i32; size];
    frontier[(offset + 1) as usize] = 0;
    for depth in 0..=distance as i32 {
        for diagonal in (-depth..=depth).step_by(2) {
            budget.work(1)?;
            let cell = (offset + diagonal) as usize;
            let insert = frontier[cell + 1];
            let delete = if frontier[cell - 1] >= 0 {
                frontier[cell - 1] + 1
            } else {
                -1
            };
            let mut x = insert.max(delete);
            let mut y = x - diagonal;
            if x < 0 || y < 0 || x as usize > a.len() || y as usize > b.len() {
                frontier[cell] = -1;
                continue;
            }
            while (x as usize) < a.len() && (y as usize) < b.len() {
                budget.work(1)?;
                if (x as usize, y as usize) == edge || a[x as usize] != b[y as usize] {
                    break;
                }
                x += 1;
                y += 1;
            }
            frontier[cell] = x;
            if x as usize == a.len() && y as usize == b.len() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
pub(super) fn script(a: &[u32], b: &[u32], budget: &mut Budget) -> Result<Script, String> {
    enum Task {
        Diff(Range<usize>, Range<usize>),
        Equal(usize, usize, usize),
    }
    let mut stack = vec![Task::Diff(0..a.len(), 0..b.len())];
    let mut steps = vec![];
    while let Some(task) = stack.pop() {
        budget.work(1)?;
        if stack.len() > 256 {
            return Err("Merge diff exceeds its partition stack budget".into());
        }
        let (mut ar, mut br) = match task {
            Task::Equal(x, y, n) => {
                append(&mut steps, Step::Equal(x, y, n))?;
                continue;
            }
            Task::Diff(a, b) => (a, b),
        };
        let mut head = 0;
        while ar.start + head < ar.end && br.start + head < br.end {
            budget.work(1)?;
            if a[ar.start + head] != b[br.start + head] {
                break;
            }
            head += 1;
        }
        if head > 0 {
            append(&mut steps, Step::Equal(ar.start, br.start, head))?;
            ar.start += head;
            br.start += head;
        }
        let mut tail = 0;
        while ar.start < ar.end - tail && br.start < br.end - tail {
            budget.work(1)?;
            if a[ar.end - tail - 1] != b[br.end - tail - 1] {
                break;
            }
            tail += 1;
        }
        if tail > 0 {
            stack.push(Task::Equal(ar.end - tail, br.end - tail, tail));
            ar.end -= tail;
            br.end -= tail;
        }
        if ar.is_empty() && br.is_empty() {
            continue;
        }
        if ar.is_empty() || br.is_empty() {
            append(&mut steps, Step::Change(ar.start, ar.end, br.start, br.end))?;
            continue;
        }
        if ar.len() == 1 {
            let found = b[br.clone()].iter().position(|v| *v == a[ar.start]);
            budget.work(br.len())?;
            if let Some(at) = found {
                let at = br.start + at;
                if br.start < at {
                    append(&mut steps, Step::Change(ar.start, ar.start, br.start, at))?;
                }
                append(&mut steps, Step::Equal(ar.start, at, 1))?;
                if at + 1 < br.end {
                    append(&mut steps, Step::Change(ar.end, ar.end, at + 1, br.end))?;
                }
            } else {
                append(&mut steps, Step::Change(ar.start, ar.end, br.start, br.end))?;
            }
            continue;
        }
        if br.len() == 1 {
            let found = a[ar.clone()].iter().position(|v| *v == b[br.start]);
            budget.work(ar.len())?;
            if let Some(at) = found {
                let at = ar.start + at;
                if ar.start < at {
                    append(&mut steps, Step::Change(ar.start, at, br.start, br.start))?;
                }
                append(&mut steps, Step::Equal(at, br.start, 1))?;
                if at + 1 < ar.end {
                    append(&mut steps, Step::Change(at + 1, ar.end, br.end, br.end))?;
                }
            } else {
                append(&mut steps, Step::Change(ar.start, ar.end, br.start, br.end))?;
            }
            continue;
        }
        if let Some((x, y)) = split(&a[ar.clone()], &b[br.clone()], budget)? {
            let (x, y) = (ar.start + x, br.start + y);
            stack.push(Task::Diff(x..ar.end, y..br.end));
            stack.push(Task::Diff(ar.start..x, br.start..y));
        } else {
            append(&mut steps, Step::Change(ar.start, ar.end, br.start, br.end))?;
        }
    }
    let (mut hunks, mut equals, mut distance) = (vec![], vec![], 0);
    let (mut old, mut new) = (0, 0);
    for step in steps {
        match step {
            Step::Equal(a, b, n) => {
                if a != old || b != new {
                    return Err("Merge equality partition is inconsistent".into());
                }
                old += n;
                new += n;
                equals.push((a..old, b..new));
            }
            Step::Change(a, b, c, d) => {
                if a != old || c != new {
                    return Err("Merge change partition is inconsistent".into());
                }
                old = b;
                new = d;
                distance += (b - a) + (d - c);
                hunks.push(Hunk {
                    old: a..b,
                    new: c..d,
                    footprint: a..b,
                    uncertain: false,
                });
            }
        }
    }
    if old != a.len() || new != b.len() || hunks.len() > MAX_ITEMS {
        return Err("Merge diff does not cover its bounded inputs".into());
    }
    Ok(Script {
        hunks,
        equals,
        distance,
    })
}
impl Script {
    pub(super) fn matched(&self, old: usize) -> Option<usize> {
        let i = self.equals.partition_point(|(a, _)| a.end <= old);
        self.equals
            .get(i)
            .and_then(|(a, b)| a.contains(&old).then(|| b.start + old - a.start))
    }
}
