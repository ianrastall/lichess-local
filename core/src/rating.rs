//! Glickman's March 2022 Glicko-2 update. The reference test is from his paper.
use crate::protocol::Rating;
const SCALE: f64 = 173.7178;

pub fn initial() -> Rating {
    Rating {
        value: 1500.0,
        deviation: 350.0,
        volatility: 0.06,
        ..Default::default()
    }
}

pub fn update(player: &Rating, games: &[(f64, f64, f64)]) -> Rating {
    let mu = (player.value - 1500.0) / SCALE;
    let phi = player.deviation / SCALE;
    if games.is_empty() {
        return Rating {
            deviation: (phi.hypot(player.volatility) * SCALE).min(350.0),
            ..player.clone()
        };
    }
    let mut information = 0.0;
    let mut improvement = 0.0;
    for &(rating, rd, score) in games {
        let opponent = (rating - 1500.0) / SCALE;
        let g = 1.0 / (1.0 + 3.0 * (rd / SCALE).powi(2) / std::f64::consts::PI.powi(2)).sqrt();
        let e = 1.0 / (1.0 + (-g * (mu - opponent)).exp());
        information += g * g * e * (1.0 - e);
        improvement += g * (score - e);
    }
    let v = 1.0 / information;
    let delta = v * improvement;
    let a = player.volatility.powi(2).ln();
    let tau: f64 = 0.5;
    let f = |x: f64| {
        let ex = x.exp();
        ex * (delta * delta - phi * phi - v - ex) / (2.0 * (phi * phi + v + ex).powi(2))
            - (x - a) / tau.powi(2)
    };
    let mut left = a;
    let mut right = if delta * delta > phi * phi + v {
        (delta * delta - phi * phi - v).ln()
    } else {
        let mut k = 1.0;
        while f(a - k * tau) < 0.0 && k < 10000.0 {
            k += 1.0;
        }
        a - k * tau
    };
    let mut fl = f(left);
    let mut fr = f(right);
    for _ in 0..1000 {
        if (right - left).abs() <= 0.000001 {
            break;
        }
        let c = left + (left - right) * fl / (fr - fl);
        let fc = f(c);
        if fc * fr <= 0.0 {
            left = right;
            fl = fr;
        } else {
            fl /= 2.0;
        }
        right = c;
        fr = fc;
    }
    let volatility = (left / 2.0).exp();
    let pre = phi.hypot(volatility);
    let next_phi = (1.0 / (1.0 / pre.powi(2) + 1.0 / v)).sqrt();
    let mut result = Rating {
        value: 1500.0 + SCALE * (mu + next_phi.powi(2) * improvement),
        deviation: (next_phi * SCALE).min(350.0),
        volatility,
        ..player.clone()
    };
    for &(_, _, score) in games {
        result.games += 1;
        if score == 1.0 {
            result.wins += 1;
        } else if score == 0.0 {
            result.losses += 1;
        } else {
            result.draws += 1;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_worked_example() {
        let r = Rating {
            value: 1500.0,
            deviation: 200.0,
            volatility: 0.06,
            ..Default::default()
        };
        let n = update(
            &r,
            &[
                (1400.0, 30.0, 1.0),
                (1550.0, 100.0, 0.0),
                (1700.0, 300.0, 0.0),
            ],
        );
        assert!((n.value - 1464.06).abs() < 0.02);
        assert!((n.deviation - 151.52).abs() < 0.02);
        assert!((n.volatility - 0.059996).abs() < 0.000001);
    }
    #[test]
    fn uncertainty_grows_without_games() {
        let r = initial();
        assert_eq!(update(&r, &[]).deviation, 350.0);
    }
}
