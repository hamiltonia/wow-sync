//! The only place that decides which member state is used (publication.md §6, M-05).

use crate::ids::{ComputerId, EnrollmentId};
use crate::layout::parse_state_name;
use crate::records::{
    decode, validate_state_for, AdmissionWatermarks, CodeState, GroupRecord, MemberRecord, Record,
    RecordError, Watermark,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFileObs {
    pub name: String,
    pub size: u64,
    pub cloud_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberObs {
    /// The member directory the observation came from.
    pub computer: ComputerId,
    pub member: Result<MemberRecord, RecordError>,
    pub files: Vec<StateFileObs>,
    /// Bytes of the highest-seq name of the current enrollment, if the runtime could read them.
    pub newest_bytes: Option<Result<Vec<u8>, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberLatest {
    Valid { seq: u64, state: Box<CodeState> },
    Unusable { seq: u64, reason: String },
    Regressed { max_seq_seen: u64 },
    InvalidMember(String),
    NoState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberView {
    pub computer: ComputerId,
    pub enrollment: EnrollmentId,
    pub display_name: String,
    pub latest: MemberLatest,
}

fn newest(m: &MemberRecord, files: &[StateFileObs]) -> Option<(u64, usize)> {
    files
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            parse_state_name(&f.name)
                .filter(|(e, _)| *e == m.enrollment_id)
                .map(|(_, s)| (s, i))
        })
        .max()
}

/// The file the runtime must read for this member: the highest sequence of its current enrollment.
pub fn newest_state_name(m: &MemberObs) -> Option<String> {
    let rec = m.member.as_ref().ok()?;
    newest(rec, &m.files).map(|(_, i)| m.files[i].name.clone())
}

fn judge(
    group: &GroupRecord,
    m: &MemberRecord,
    o: &MemberObs,
    seq: u64,
    file: &StateFileObs,
) -> Result<Box<CodeState>, String> {
    if file.cloud_only {
        return Err("the file is not downloaded yet".into());
    }
    if file.size > CodeState::LIMIT as u64 {
        return Err("the file is too large".into());
    }
    let bytes = match &o.newest_bytes {
        Some(Ok(b)) => b,
        Some(Err(e)) => return Err(format!("unreadable: {e}")),
        None => return Err("not read".into()),
    };
    let state: CodeState = decode(bytes, CodeState::LIMIT).map_err(|e| e.to_string())?;
    validate_state_for(&state, group, &m.computer_id, &m.enrollment_id, seq)?;
    Ok(Box::new(state))
}

pub fn admit_member_states(
    group: &GroupRecord,
    obs: &[MemberObs],
    marks: &AdmissionWatermarks,
) -> (Vec<MemberView>, AdmissionWatermarks) {
    let mut out = marks.clone();
    out.group_id = group.group_id.clone();
    let mut views = Vec::new();
    for o in obs {
        let zero = EnrollmentId::parse(&"0".repeat(32)).expect("valid id");
        let invalid = |why: String| MemberView {
            computer: o.computer.clone(),
            enrollment: zero.clone(),
            display_name: o.computer.to_string(),
            latest: MemberLatest::InvalidMember(why),
        };
        let m = match &o.member {
            Err(e) => {
                views.push(invalid(e.to_string()));
                continue;
            }
            Ok(m) => m,
        };
        if m.group_id != group.group_id || m.computer_id != o.computer || m.product != group.product
        {
            views.push(invalid(
                "member record does not belong to this group, computer or product".into(),
            ));
            continue;
        }
        let old = marks.get(&m.computer_id, &m.enrollment_id).cloned();
        let old_max = old.as_ref().map_or(0, |w| w.max_seq_seen);
        let mut wm = old.unwrap_or(Watermark {
            computer_id: m.computer_id.clone(),
            enrollment_id: m.enrollment_id.clone(),
            max_seq_seen: 0,
            admitted_seq: None,
        });
        let latest = match newest(m, &o.files) {
            None if old_max > 0 => MemberLatest::Regressed {
                max_seq_seen: old_max,
            },
            None => MemberLatest::NoState,
            Some((seq, _)) if seq < old_max => MemberLatest::Regressed {
                max_seq_seen: old_max,
            },
            Some((seq, i)) => {
                wm.max_seq_seen = wm.max_seq_seen.max(seq);
                match judge(group, m, o, seq, &o.files[i]) {
                    Ok(state) => {
                        wm.admitted_seq = Some(seq);
                        MemberLatest::Valid { seq, state }
                    }
                    Err(reason) => MemberLatest::Unusable { seq, reason },
                }
            }
        };
        out.members
            .retain(|w| !(w.computer_id == wm.computer_id && w.enrollment_id == wm.enrollment_id));
        if wm.max_seq_seen > 0 || wm.admitted_seq.is_some() {
            out.members.push(wm);
        }
        views.push(MemberView {
            computer: m.computer_id.clone(),
            enrollment: m.enrollment_id.clone(),
            display_name: m.display_name.clone(),
            latest,
        });
    }
    out.members.sort_by(|a, b| {
        (&a.computer_id, &a.enrollment_id).cmp(&(&b.computer_id, &b.enrollment_id))
    });
    views.sort_by(|a, b| a.computer.cmp(&b.computer));
    (views, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::state_file_name;
    use crate::records::encode;
    use crate::testkit;

    fn state_bytes(n: u8, seq: u64) -> Vec<u8> {
        let inv = testkit::inventory(vec![testkit::addon("Suite", Some("1.0"), "a")]);
        encode(&testkit::member_state(&testkit::computer(n), seq, &inv))
    }

    fn obs(n: u8, seqs: &[u64], newest: Option<Result<Vec<u8>, String>>) -> MemberObs {
        let m = testkit::member_record(&testkit::computer(n));
        let files = seqs
            .iter()
            .map(|s| StateFileObs {
                name: state_file_name(&m.enrollment_id, *s),
                size: 100,
                cloud_only: false,
            })
            .collect();
        MemberObs {
            computer: testkit::computer(n),
            member: Ok(m),
            files,
            newest_bytes: newest,
        }
    }

    fn run(o: MemberObs, marks: &AdmissionWatermarks) -> (MemberView, AdmissionWatermarks) {
        let (mut v, m) = admit_member_states(&testkit::group_record(), &[o], marks);
        (v.remove(0), m)
    }

    fn empty() -> AdmissionWatermarks {
        AdmissionWatermarks::empty(testkit::group_id())
    }

    #[test]
    fn newest_valid_state_admitted() {
        let (v, marks) = run(obs(1, &[1, 2, 3], Some(Ok(state_bytes(1, 3)))), &empty());
        assert!(matches!(v.latest, MemberLatest::Valid { seq: 3, .. }));
        assert_eq!(
            (marks.members[0].max_seq_seen, marks.members[0].admitted_seq),
            (3, Some(3))
        );
        assert_eq!(
            run(obs(1, &[], None), &empty()).0.latest,
            MemberLatest::NoState
        );
    }

    #[test]
    fn newest_malformed_no_fallback() {
        let mut prior = empty();
        prior.members.push(Watermark {
            computer_id: testkit::computer(1),
            enrollment_id: testkit::enrollment(1),
            max_seq_seen: 2,
            admitted_seq: Some(2),
        });
        let (v, marks) = run(obs(1, &[2, 3], Some(Ok(b"{ not json".to_vec()))), &prior);
        assert!(
            matches!(v.latest, MemberLatest::Unusable { seq: 3, .. }),
            "{v:?}"
        );
        assert_eq!(marks.members[0].max_seq_seen, 3);
        assert_eq!(marks.members[0].admitted_seq, Some(2));
        let (v, _) = run(obs(1, &[3], Some(Ok(state_bytes(1, 2)))), &empty());
        assert!(
            matches!(v.latest, MemberLatest::Unusable { seq: 3, .. }),
            "body seq disagrees with the name"
        );
    }

    #[test]
    fn newest_unsupported_major_no_fallback() {
        let text = String::from_utf8(state_bytes(1, 3))
            .unwrap()
            .replace("wowsync.code-state/1", "wowsync.code-state/2");
        let (v, _) = run(obs(1, &[2, 3], Some(Ok(text.into_bytes()))), &empty());
        match v.latest {
            MemberLatest::Unusable { seq: 3, reason } => {
                assert!(reason.contains("newer"), "{reason}")
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn newest_cloud_only_no_fallback() {
        let mut o = obs(1, &[2, 3], None);
        o.files[1].cloud_only = true;
        let (v, _) = run(o, &empty());
        assert!(matches!(v.latest, MemberLatest::Unusable { seq: 3, .. }));
        let mut big = obs(1, &[3], Some(Ok(state_bytes(1, 3))));
        big.files[0].size = CodeState::LIMIT as u64 + 1;
        assert!(matches!(
            run(big, &empty()).0.latest,
            MemberLatest::Unusable { seq: 3, .. }
        ));
    }

    #[test]
    fn regression_after_seeing_newer() {
        let (_, marks) = run(
            obs(1, &[1, 2, 3, 4, 5], Some(Ok(state_bytes(1, 5)))),
            &empty(),
        );
        let (v, after) = run(obs(1, &[3, 4], Some(Ok(state_bytes(1, 4)))), &marks);
        assert_eq!(v.latest, MemberLatest::Regressed { max_seq_seen: 5 });
        assert_eq!(after.members[0].max_seq_seen, 5);
        let (v, _) = run(obs(1, &[], None), &marks);
        assert_eq!(v.latest, MemberLatest::Regressed { max_seq_seen: 5 });
    }

    #[test]
    fn enrollment_change_resets_watermark() {
        let (_, marks) = run(obs(1, &[9], Some(Ok(state_bytes(1, 9)))), &empty());
        let mut m = testkit::member_record(&testkit::computer(1));
        m.enrollment_id = testkit::enrollment(77);
        let mut st = testkit::member_state(&testkit::computer(1), 1, &testkit::inventory(vec![]));
        st.enrollment_id = m.enrollment_id.clone();
        let o = MemberObs {
            computer: testkit::computer(1),
            files: vec![StateFileObs {
                name: state_file_name(&m.enrollment_id, 1),
                size: 10,
                cloud_only: false,
            }],
            member: Ok(m),
            newest_bytes: Some(Ok(encode(&st))),
        };
        let (v, after) = run(o, &marks);
        assert!(matches!(v.latest, MemberLatest::Valid { seq: 1, .. }));
        assert_eq!(after.members.len(), 2, "old enrollment kept for audit");
        assert!(after.members.iter().any(|w| w.max_seq_seen == 9));
    }

    #[test]
    fn only_newest_name_is_read() {
        let mut o = obs(1, &[1, 7, 3], None);
        let other = testkit::member_record(&testkit::computer(2)).enrollment_id;
        o.files.push(StateFileObs {
            name: state_file_name(&other, 99),
            size: 1,
            cloud_only: false,
        });
        o.files.push(StateFileObs {
            name: format!("{}-0000000050.json.tmp", testkit::enrollment(1)),
            size: 1,
            cloud_only: false,
        });
        assert_eq!(
            newest_state_name(&o),
            Some(state_file_name(&testkit::enrollment(1), 7))
        );
        o.member = Err(RecordError::SchemaMissing);
        assert_eq!(newest_state_name(&o), None);
    }
}
