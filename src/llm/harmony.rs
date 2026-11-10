//! Harmony / dissonance (P1, Comparative-Thinker adaptation plan §3):
//! confidence as agreement between the search pillars.
//!
//! DERIVE-class (S4): normalised variance over N values in [0,1] is
//! textbook statistics — max variance for N values is (N-1)/N (one 1,
//! N-1 zeros), so dissonance = Σ(sᵢ − s̄)² / ((N−1)/N), clamped to [0,1].
//! N=3 gives 2/3, the constant the source experiment hard-coded. The
//! GTW generalisation is the same formula with N as a parameter, because
//! the graph pillar is OPTIONAL (no graph on disk → two pillars).
//!
//! The measured characterization of the source run (S3,
//! `/tmp/opencode/ct-091-characterization.txt`) is pinned in the tests —
//! the reference numbers are MEASURED, not invented.
//!
//! This module is pure and always compiled; the wiring lives in the
//! sense pipeline (feature-gated — see the adaptation plan §3.2).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Harmony {
    /// 1 - dissonance, 0..=1
    pub score: f32,
    /// Normalised variance over the AVAILABLE pillars, 0..=1
    pub dissonance: f32,
    pub pillars: HarmonyPillars,
    /// How many pillars actually participated (2 = no graph on disk).
    /// One pillar cannot disagree with itself: neutral, never faked 1.0.
    pub pillars_available: usize,
    pub verdict: HarmonyVerdict,
    /// One sentence explaining the disagreement — None when the pillars
    /// agree. A number without a reason drowns in an agent's noise.
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarmonyPillars {
    pub semantic: f32,
    pub lexical: Option<f32>,
    pub graph: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarmonyVerdict {
    Harmonious,
    MinorDissonance,
    Disharmonious,
}

pub const DISSONANCE_MINOR: f32 = 0.25;
pub const DISSONANCE_HIGH: f32 = 0.50;

#[derive(Debug, Clone, Copy)]
pub struct Pillars {
    pub semantic: f32,
    pub lexical: Option<f32>,
    pub graph: Option<f32>,
}

impl From<&Pillars> for HarmonyPillars {
    fn from(p: &Pillars) -> Self {
        HarmonyPillars {
            semantic: p.semantic,
            lexical: p.lexical,
            graph: p.graph,
        }
    }
}

/// Normalised variance over the available pillars (DERIVE: max variance
/// of N values in [0,1] is (N-1)/N). Single pillar → neutral 0.0.
pub fn dissonance_of(scores: &[f32]) -> f32 {
    let n = scores.len();
    if n < 2 {
        return 0.0;
    }
    let mean = scores.iter().sum::<f32>() / n as f32;
    let sum_sq: f32 = scores.iter().map(|x| (x - mean).powi(2)).sum::<f32>();
    let max_var = (n - 1) as f32 / n as f32;
    (sum_sq / max_var).clamp(0.0, 1.0)
}

pub fn harmony(p: &Pillars) -> Harmony {
    let mut s = vec![p.semantic.clamp(0.0, 1.0)];
    if let Some(l) = p.lexical {
        s.push(l.clamp(0.0, 1.0));
    }
    if let Some(g) = p.graph {
        s.push(g.clamp(0.0, 1.0));
    }
    let dissonance = dissonance_of(&s);

    // EN kanal kan inte vara oense med sig själv: neutral, aldrig fejkad
    // 1.0 (plan §3.3) — score = rå pelare, aldrig uppluftad till konfidens.
    if s.len() < 2 {
        return Harmony {
            score: s[0],
            dissonance: 0.0,
            pillars: HarmonyPillars::from(p),
            pillars_available: s.len(),
            verdict: HarmonyVerdict::Harmonious,
            suggestion: None,
        };
    }

    let verdict = if dissonance >= DISSONANCE_HIGH {
        HarmonyVerdict::Disharmonious
    } else if dissonance >= DISSONANCE_MINOR {
        HarmonyVerdict::MinorDissonance
    } else {
        HarmonyVerdict::Harmonious
    };
    Harmony {
        score: 1.0 - dissonance,
        dissonance,
        pillars: HarmonyPillars::from(p),
        pillars_available: s.len(),
        verdict,
        suggestion: suggestion_for(verdict, p),
    }
}

/// The explanation is what makes the measure usable — a number without a
/// cause drowns in an agent's noise (cf. suspect_reason: binary, no grad).
fn suggestion_for(v: HarmonyVerdict, p: &Pillars) -> Option<String> {
    let mut named: Vec<(&str, f32)> = vec![("semantic", p.semantic)];
    if let Some(l) = p.lexical {
        named.push(("lexical", l));
    }
    if let Some(g) = p.graph {
        named.push(("graph", g));
    }
    let (strong, s_val) = named
        .iter()
        .cloned()
        .fold(("", f32::MIN), |a, b| if b.1 > a.1 { b } else { a });
    let (weak, w_val) = named
        .iter()
        .cloned()
        .fold(("", f32::MAX), |a, b| if b.1 < a.1 { b } else { a });

    match v {
        HarmonyVerdict::Harmonious => None,
        HarmonyVerdict::MinorDissonance => Some(format!(
            "channels partly disagree ({} {:.2} vs {} {:.2}) — verify with read_node before trusting the top hit",
            strong, s_val, weak, w_val
        )),
        HarmonyVerdict::Disharmonious => Some(format!(
            "{} is confident ({:.2}) while {} is not ({:.2}); the match may be topical rather than structural",
            strong, s_val, weak, w_val
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    // ---- §3.5 contract tests (written before the implementation) ----

    #[test]
    fn harmony_single_pillar_is_neutral_not_confident() {
        let h = harmony(&Pillars {
            semantic: 0.9,
            lexical: None,
            graph: None,
        });
        assert_eq!(h.pillars_available, 1);
        assert!(approx(h.dissonance, 0.0));
        assert!(approx(h.score, 0.9), "score == pillar, never faked 1.0");
    }

    #[test]
    fn harmony_two_agreeing_pillars_is_one() {
        let h = harmony(&Pillars {
            semantic: 0.9,
            lexical: Some(0.9),
            graph: None,
        });
        assert!(approx(h.dissonance, 0.0));
        assert!(approx(h.score, 1.0));
    }

    #[test]
    fn harmony_two_max_disagreement_is_zero() {
        let h = harmony(&Pillars {
            semantic: 1.0,
            lexical: Some(0.0),
            graph: None,
        });
        assert!(approx(h.dissonance, 1.0));
        assert!(approx(h.score, 0.0));
    }

    #[test]
    fn harmony_missing_graph_is_not_agreement() {
        // Honest neutrality: the 2-pillar answer is computed on the two
        // REMAINING pillars only — no imputation, no absence-as-agreement.
        // A DISSENTING third pillar (graph: Some(0.0)) legitimately lowers
        // the score further; absence does not. Either way the response
        // declares what participated (pillars_available) — the consumer
        // decides how much to trust a 2-pillar verdict.
        let without = harmony(&Pillars {
            semantic: 0.9,
            lexical: Some(0.1),
            graph: None,
        });
        let with_zero = harmony(&Pillars {
            semantic: 0.9,
            lexical: Some(0.1),
            graph: Some(0.0),
        });
        assert!(
            approx(without.dissonance, 0.64),
            "2-pillar normalised variance"
        );
        assert!(
            approx(with_zero.dissonance, 0.73),
            "dissenting third pillar adds spread"
        );
        assert_eq!(without.pillars_available, 2);
        assert_eq!(with_zero.pillars_available, 3);
        assert!(
            without.suggestion.is_some(),
            "2-pillar dissonance still explains itself"
        );
    }

    // ---- S3 characterization pins (MEASURED source run, 2026-11-10) ----

    #[test]
    fn harmony_matches_measured_ct_characterization() {
        // From /tmp/opencode/ct-091-characterization.txt — every value
        // below is a MEASURED output of the source binary, not invented.
        let case = |s: Pillars| {
            dissonance_of(&{
                let mut v = vec![s.semantic];
                if let Some(l) = s.lexical {
                    v.push(l);
                }
                if let Some(g) = s.graph {
                    v.push(g);
                }
                v
            })
        };

        assert!(approx(
            case(Pillars {
                semantic: 0.9,
                lexical: Some(0.8),
                graph: Some(0.95)
            }),
            0.0175
        ));
        assert!(approx(
            case(Pillars {
                semantic: 0.7,
                lexical: Some(0.8),
                graph: Some(0.6)
            }),
            0.0300
        ));
        assert!(approx(
            case(Pillars {
                semantic: 0.9,
                lexical: Some(0.1),
                graph: Some(0.2)
            }),
            0.5700
        ));
        assert!(approx(
            case(Pillars {
                semantic: 0.2,
                lexical: Some(0.3),
                graph: Some(0.9)
            }),
            0.4300
        ));
        assert!(approx(
            case(Pillars {
                semantic: 0.7,
                lexical: Some(0.7),
                graph: Some(0.7)
            }),
            0.0
        ));
        assert!(approx(
            case(Pillars {
                semantic: 0.0,
                lexical: Some(1.0),
                graph: Some(0.0)
            }),
            1.0
        ));
    }

    #[test]
    fn harmony_three_matches_derived_reference() {
        // DERIVE cross-check (plan §3.1): [0.9, 0.2, 0.1] — mean 0.4,
        // Σsq = 0.38, normalised = 0.38 / (2/3) = 0.57.
        let h = harmony(&Pillars {
            semantic: 0.9,
            lexical: Some(0.2),
            graph: Some(0.1),
        });
        assert!(approx(h.dissonance, 0.57));
        assert_eq!(h.verdict, HarmonyVerdict::Disharmonious);
    }

    #[test]
    fn harmony_verdict_thresholds() {
        // [0.8, 0.3, 0.2]: mean 0.4333, Σsq ≈ 0.2067 → d ≈ 0.31 —
        // comfortably inside the MINOR band (0.25..0.50).
        let h_mid = harmony(&Pillars {
            semantic: 0.8,
            lexical: Some(0.3),
            graph: Some(0.2),
        });
        assert!(
            approx(h_mid.dissonance, 0.31),
            "comfortably inside the MINOR band"
        );
        assert_eq!(h_mid.verdict, HarmonyVerdict::MinorDissonance);
        let h_low = harmony(&Pillars {
            semantic: 0.9,
            lexical: Some(0.8),
            graph: Some(0.85),
        });
        assert_eq!(h_low.verdict, HarmonyVerdict::Harmonious);
        let h_high = harmony(&Pillars {
            semantic: 0.9,
            lexical: Some(0.1),
            graph: Some(0.2),
        });
        assert_eq!(h_high.verdict, HarmonyVerdict::Disharmonious);
    }

    #[test]
    fn harmony_suggestion_names_strong_and_weak_pillar() {
        let h = harmony(&Pillars {
            semantic: 0.9,
            lexical: Some(0.1),
            graph: Some(0.2),
        });
        let s = h.suggestion.expect("disharmonious must explain itself");
        assert!(s.contains("semantic"), "names the strong pillar");
        assert!(s.contains("lexical"), "names the weak pillar");
        let harmonious = harmony(&Pillars {
            semantic: 0.9,
            lexical: Some(0.85),
            graph: Some(0.9),
        });
        assert!(
            harmonious.suggestion.is_none(),
            "harmony needs no explanation"
        );
    }
}
