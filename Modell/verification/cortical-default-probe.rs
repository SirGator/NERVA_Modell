use nerva_vm::model::{Model,ModelConfig,KeyboardLayout,SensorConfig,SpikeCoding};
use nerva_qemu::{capture::Frame,keyboard::Key};
fn main(){
 for size in [32,128]{
  let mut model=Model::with_sensor(ModelConfig{processing_neurons:size,keyboard:KeyboardLayout::Wasd,..Default::default()},SensorConfig{resolution:8,coding:SpikeCoding::Deterministic,..Default::default()}).unwrap();
  println!("case={size} initial={:#?}",model.network());
  for step in 0..4{
   let values:Vec<_>=(0..64).flat_map(|i|[if step==0 {0}else if step==1 || (i+step)%3==0 {255}else{0};3]).collect();
   let keys=if step%2==0{[Key::A]}else{[Key::D]};
   model.record_executed_keys(&keys).unwrap();
   let outcome=model.step(&Frame::new(8,8,values).unwrap()).unwrap();
   println!("step={outcome:?} network={:#?}",model.network());
  }
 }
}
