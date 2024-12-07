use bevy::math::bounding::Aabb2d;

pub trait Bounded2d
{
    fn aabb(&self) -> Aabb2d;
}
