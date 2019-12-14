pub trait OnEraEnding<ValidatorId, EraIndex> {
	fn on_era_ending(
		_ending: EraIndex,
		_start_era: EraIndex,
	) -> Option<Vec<ValidatorId>>;
}
