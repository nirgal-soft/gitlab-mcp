use rand::Rng;
use rmcp::model::{ErrorData as McpError, CallToolResult, Content};
use serde::Deserialize;
use schemars::JsonSchema;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RollRequestExample{
  /// Number of sides on the dice (e.g. 6 for d6, 20 for d20)
  pub sides: u32,
  /// Number of dice to roll
  #[serde(default = "default_count")]
  pub count: u32,
}

fn default_count() -> u32{1}

#[derive(Clone)]
pub struct DiceToolExample;

impl DiceToolExample{
  pub async fn roll(&self, req: RollRequestExample) -> Result<CallToolResult, McpError>{
    if req.sides == 0{
      return Err(McpError::invalid_params("Dice must have at least 1 side", None));
    }
    if req.count == 0 || req.count > 100{
      return Err(McpError::invalid_params("Count must be between 1 and 100", None));
    }

    let mut rng = rand::rng();
    let rolls: Vec<u32> = (0..req.count)
      .map(|_| rng.random_range(1..=req.sides))
      .collect();

    let total: u32 = rolls.iter().sum();

    let result_text = if req.count == 1{
      format!("Rolled a d{}: {}", req.sides, rolls[0])
    }else{
      format!(
        "Rolled {}d{}: {} (total: {})",
        req.count,
        req.sides,
        rolls.iter().map(|r| r.to_string()).collect::<Vec<String>>().join(", "),
        total
      )
    };

    Ok(CallToolResult::success(vec![Content::text(result_text)]))
  }
}
