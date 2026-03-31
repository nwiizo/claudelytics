---
name: orchestrator
description: Split complex tasks into sequential steps with parallel subtasks. Use for multi-step workflows needing dependency ordering and adaptive planning.
---

# Orchestrator

Split the following task into sequential steps with parallel subtasks: $ARGUMENTS

## Process

### 1. Initial Analysis
- Analyze the entire task scope and requirements
- Identify dependencies and execution order
- Plan 2-4 sequential steps based on dependencies

### 2. Step Planning
- Each step can contain multiple parallel subtasks
- Define what context from previous steps is needed
- Request concise summaries (100-200 words) from each subtask

### 3. Step-by-Step Execution
- Execute all subtasks within a step in parallel using Agent tool
- Wait for all subtasks in current step to complete
- Pass relevant results to next step

### 4. Review and Adapt After Each Step
- Validate if remaining steps are still appropriate
- Adjust next steps based on discoveries
- Add, remove, or modify subtasks as needed

### 5. Progressive Aggregation
- Synthesize results from completed step
- Use synthesized results as context for next step
- Build comprehensive understanding progressively

## Adaptive Planning

After each step, explicitly reconsider:
- Are the next steps still relevant?
- Did we discover something requiring new tasks?
- Can we skip or simplify upcoming steps?
- Should we add new validation steps?

```
Initial Plan: Step 1 -> Step 2 -> Step 3 -> Step 4
After Step 2 (no errors found): Skip Step 3 -> Simplified Step 4
After Step 2 (critical issue): -> New Step 2.5 -> Modified Step 3
```

## Guidelines

- Always start with a single analysis task to understand full scope
- Group related parallel tasks within the same step
- Pass only essential findings between steps (summaries, not full output)
- Use TaskCreate/TaskUpdate to track steps and subtasks
