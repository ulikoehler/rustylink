use rustylink::model::{Mask, MaskParameter, MaskParamType, Block};
use rustylink::mask_eval::evaluate_mask_display;
use std::collections::BTreeMap;

#[test]
fn test_eval_simple() {
    let mut block = Block {
        block_type: "SubSystem".into(), name: "Test".into(), sid: None, position: None, zorder: None,
        commented: false, is_matlab_function: false, properties: BTreeMap::new(), ports: vec![], subsystem: None,
        c_function: None, instance_data: None, mask: Some(Mask { display: Some("disp(mytab{control})".into()), description: None, initialization: Some("mytab={'Position','Zero Torque','OFF'};".into()), help: None, parameters: vec![MaskParameter { name: "control".into(), param_type: MaskParamType::Popup, prompt: None, value: Some("1. Position".into()), callback: None, tunable: None, visible: None, type_options: vec![] }], dialog: vec![] }), annotations: vec![], background_color: None, show_name: None, font_size: None, font_weight: None, mask_display_text: None };
    evaluate_mask_display(&mut block);
    assert_eq!(block.mask_display_text.as_deref(), Some("Position"));
}
