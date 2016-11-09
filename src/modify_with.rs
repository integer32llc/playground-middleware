use iron::prelude::*;
use iron::AfterMiddleware;
use iron::modifier::Modifier;

/// Applies a modifier to every request
pub struct ModifyWith<M> {
    modifier: M,
}

impl<M> ModifyWith<M> {
    pub fn new(modifier: M) -> ModifyWith<M> {
        ModifyWith {
            modifier: modifier,
        }
    }
}

impl<M> AfterMiddleware for ModifyWith<M>
    where M: Clone + Modifier<Response> + Send + Sync + 'static
{
    fn after(&self, _req: &mut Request, mut res: Response) -> IronResult<Response> {
        self.modifier.clone().modify(&mut res);
        Ok(res)
    }
}
