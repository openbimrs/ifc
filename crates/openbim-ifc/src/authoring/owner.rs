//! An owner history, through `ifc-author`'s writers.
//!
//! `ifc-author` builds `IfcPerson`, `IfcOrganization`,
//! `IfcPersonAndOrganization`, `IfcApplication` and `IfcOwnerHistory` and
//! enforces their own rules (a person names someone, a known change action,
//! no modification before creation). It stages them on a transaction of its
//! own; this module adopts them into the batch under the batch's ids and
//! checks each against the declared release, which is where IFC2X3's
//! required `ChangeAction` is enforced.

use std::collections::HashMap;

use ifc_author::{
    add_application, add_organization, add_owner_history, add_person, add_person_and_organization,
    ApplicationDraft, OrganizationDraft, OwnerHistoryDraft, PersonDraft,
};
use ifc_model::{Edit, Entity, EntityId, Transaction, Value};

use super::check::{recheck, relations};
use super::plan::Planner;
use super::{AuthoringFailure as F, OwnerHistoryOp};

impl Planner<'_, '_> {
    pub(super) fn owner_history(&mut self, op: &OwnerHistoryOp) -> Result<EntityId, F> {
        let mut tx = Transaction::new(self.view.base());
        let mut person = PersonDraft::new();
        if let Some(value) = &op.person_identification {
            person = person.identification(value);
        }
        if let Some(value) = &op.family_name {
            person = person.family_name(value);
        }
        if let Some(value) = &op.given_name {
            person = person.given_name(value);
        }
        let person = add_person(&mut tx, person).map_err(F::Author)?;
        let organization = add_organization(&mut tx, OrganizationDraft::new(&op.organization))
            .map_err(F::Author)?;
        let user = add_person_and_organization(&mut tx, person, organization);
        let application = add_application(
            &mut tx,
            ApplicationDraft::new(
                organization,
                &op.application_version,
                &op.application_name,
                &op.application_identifier,
            ),
        )
        .map_err(F::Author)?;
        let mut draft = OwnerHistoryDraft::new(user, application, op.creation_date);
        if let Some(action) = &op.change_action {
            draft = draft.change_action(action);
        }
        if let Some(modified) = op.last_modified_date {
            draft = draft.last_modified_date(modified);
        }
        let history = add_owner_history(&mut tx, draft).map_err(F::Author)?;
        self.adopt(&tx, history)
    }

    /// Move the entities `tx` stages into the batch under fresh ids,
    /// checking each; returns the new id of `primary`.
    fn adopt(&mut self, tx: &Transaction, primary: EntityId) -> Result<EntityId, F> {
        let staged: Vec<(EntityId, &Entity)> = tx
            .edits()
            .iter()
            .filter_map(|edit| match edit {
                Edit::Create { id, entity } => Some((*id, entity)),
                _ => None,
            })
            .collect();
        let ids: HashMap<EntityId, EntityId> = staged
            .iter()
            .map(|(temporary, _)| (*temporary, self.view.allocate()))
            .collect();
        for (temporary, entity) in staged {
            let mut entity = entity.clone();
            for value in &mut entity.attributes {
                if let Value::Ref(id) = value {
                    if let Some(adopted) = ids.get(id) {
                        *id = *adopted;
                    }
                }
            }
            recheck(self.schema, &entity)?;
            relations(self.schema, &self.view, &entity, None)?;
            self.view.create(ids[&temporary], entity);
        }
        Ok(ids[&primary])
    }
}
