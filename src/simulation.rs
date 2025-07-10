use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, derive_new::new)]
pub struct SimulationObject {
    #[serde(rename = "name")]
    pub id_name: String,
    #[serde(rename = "compute values")]
    pub physics_data: SimulationPhysicsObject,
    #[serde(rename = "enter simulation values")]
    pub enter_configuration: SimulationEnterConfiguration,
}

#[derive(Serialize, Deserialize, Debug, derive_new::new)]
pub struct SimulationPhysicsObject {
    #[serde(rename = "mass")]
    pub simulation_body_mass: f32,
    #[serde(rename = "radius")]
    pub simulation_body_radius: f32,
}

#[derive(Serialize, Deserialize, Debug, derive_new::new)]
pub struct SimulationEnterConfiguration {
    #[serde(rename = "enter speed")]
    pub simulation_enter_speed: [f32; 3],
    #[serde(rename = "enter position")]
    pub simulation_enter_position: [f32; 3],
}
