use udf::prelude::*;

#[derive(Debug, PartialEq)]
struct RunningTotal(i64);

#[register]
impl BasicUdf for RunningTotal {
    type Returns<'a> = i64;

    fn init(_cfg: &UdfCfg<Init>, args: &ArgList<Init>) -> Result<Self, String> {
        if args.len() != 1 {
            return Err(format!("expected 1 argument; got {}", args.len()));
        }

        // Coerce everything to an integer
        args.get(0).unwrap().set_type_coercion(SqlType::Int);

        Ok(Self(0))
    }

    fn process<'a>(
        &'a mut self,
        _cfg: &UdfCfg<Process>,
        args: &ArgList<Process>,
        _error: Option<NonZeroU8>,
    ) -> Result<Self::Returns<'a>, ProcessError> {
        // Get the value as an integer and add it to our total
        self.0 += args.get(0).unwrap().value().as_int().unwrap_or(0);

        // The result is just our running total
        Ok(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use udf::mock::*;

    #[test]
    fn test_wrong_args() {
        let mut cfg = MockUdfCfg::new();
        let mut arglist = mock_args![]; // empty
        let res = RunningTotal::init(cfg.as_init(), arglist.as_init());

        assert_eq!(res, Err("expected 1 argument; got 0".to_owned()));
    }

    #[test]
    fn test_single() {
        let mut cfg = MockUdfCfg::new();
        let mut arglist = mock_args![(10, "", false)];
        let mut rt = RunningTotal::init(cfg.as_init(), arglist.as_init()).unwrap();
        let res = rt.process(cfg.as_process(), arglist.as_process(), None);

        assert_eq!(res, Ok(10));
    }

    #[test]
    fn test_null() {
        // We need to verify that we handle null variables correctly
        let mut cfg = MockUdfCfg::new();
        let mut row_args = [
            mock_args![(Int None, "", false)],
            mock_args![(10, "", false)],
            mock_args![(Int None, "", false)],
            mock_args![(-20, "", false)],
        ];
        let mut rt = RunningTotal::init(cfg.as_init(), row_args[0].as_init()).unwrap();

        let outputs = [0i64, 10, 10, -10];

        for (arglist, outval) in row_args.iter_mut().zip(outputs.iter()) {
            let res = rt.process(cfg.as_process(), arglist.as_process(), None);
            assert_eq!(res, Ok(*outval));
        }
    }

    #[test]
    fn test_multiple() {
        let mut cfg = MockUdfCfg::new();
        let mut arglist = mock_args![(0, "", false)];
        let mut rt = RunningTotal::init(cfg.as_init(), arglist.as_init()).unwrap();

        let inputs = [10i64, 20, -4, 100, -50, 0];
        let outputs = [10i64, 30, 26, 126, 76, 76];

        for (inval, outval) in inputs.iter().zip(outputs.iter()) {
            let mut arglist = mock_args![(*inval, "", false)];
            let res = rt.process(cfg.as_process(), arglist.as_process(), None);

            assert_eq!(res, Ok(*outval));
        }
    }
}
