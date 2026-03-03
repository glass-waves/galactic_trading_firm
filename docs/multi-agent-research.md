# Multi-agent LLM systems for financial trading: 10 key papers

The research landscape for multi-agent LLM trading systems has exploded since mid-2024, with several architecturally distinct approaches now validated through backtesting. **The most actionable finding across this literature is that decomposing financial analysis into specialized agents—mimicking real trading firm structures—consistently outperforms single-agent and traditional baselines on risk-adjusted returns.** A parallel line of work on multi-agent orchestration theory provides critical guardrails: formal consensus protocols, empirical failure taxonomies, and evidence that dynamic orchestration outperforms static workflows. Together, these 10 papers form a complete knowledge base for designing a multi-agent intra-day trading system, covering architecture, specialization, coordination, and known failure modes.

---

## 1. QuantAgent tackles high-frequency trading with price-only multi-agent reasoning

**Title:** QuantAgent: Price-Driven Multi-Agent LLMs for High-Frequency Trading
**Authors:** Fei Xiong, Xiang Zhang, Aosong Feng, Siqi Sun, Chenyu You
**Date:** September 12, 2025 (revised September 27, 2025)
**Venue:** arXiv preprint (cs.CE), arXiv:2509.09995
**URL:** https://arxiv.org/abs/2509.09995

This is the single most relevant paper for an intra-day price action system. QuantAgent is the first multi-agent LLM framework designed explicitly for high-frequency trading that operates **solely on price-derived market signals**—OHLC data, technical indicators, and chart patterns—with no news or sentiment inputs. The architecture decomposes technical analysis into four specialized agents: an **Indicator Agent** (momentum oscillators like RSI, MACD), a **Pattern Agent** (candlestick pattern recognition and chart formations), a **Trend Agent** (trend/channel analysis), and a **Risk Agent** (risk-reward assessment). Each agent uses domain-specific tools and structured LLM reasoning to analyze short temporal windows. Built on LangGraph, the system was tested across nine financial instruments including Bitcoin and Nasdaq futures at **1-hour and 4-hour intervals**, consistently outperforming neural and rule-based baselines in predictive accuracy. The paper's core argument—that technical analysis is a structured, short-horizon reasoning problem ideally suited for LLM capabilities—provides theoretical grounding for the entire approach.

---

## 2. TradingAgents replicates trading firm hierarchy with bull-bear debate

**Title:** TradingAgents: Multi-Agents LLM Financial Trading Framework
**Authors:** Yijia Xiao, Edward Sun, Di Luo, Wei Wang (UCLA, MIT)
**Date:** December 28, 2024 (revised June 3, 2025, v7)
**Venue:** arXiv preprint (q-fin.TR), arXiv:2412.20138; Oral presentation at "Multi-Agent AI in the Real World" workshop
**URL:** https://arxiv.org/abs/2412.20138

TradingAgents is the most comprehensive open-source reference implementation for multi-agent financial trading. It deploys **seven specialized LLM-powered roles**: Fundamentals Analyst, Sentiment Analyst, Technical Analyst, News Analyst, Bull Researcher, Bear Researcher, Trader, and Risk Manager. The most architecturally distinctive feature is the **structured Bull-Bear debate**: two researcher agents argue opposing market positions, generating a dialectical analysis that the Trader agent synthesizes alongside historical data. The Risk Management team monitors portfolio exposure and can veto trades. Communication uses a hybrid approach combining structured reports (inspired by MetaGPT's SOP paradigm) with natural language dialogue for the debate phases. Backtesting on U.S. equities (AAPL, NVDA, MSFT, META, GOOGL) during January–March 2024 showed significant improvements in cumulative returns, **Sharpe ratio**, and maximum drawdown versus Buy-and-Hold, single-agent LLMs, MACD, RSI strategies, and RL-based approaches. The framework supports multiple LLM backends (GPT-4o, Claude, Gemini) and is open-sourced at GitHub/TauricResearch.

---

## 3. FinCon's verbal reinforcement lets agents learn from trading mistakes without retraining

**Title:** FinCon: A Synthesized LLM Multi-Agent System with Conceptual Verbal Reinforcement for Enhanced Financial Decision Making
**Authors:** Yangyang Yu, Zhiyuan Yao, Haohang Li, Zhiyang Deng, Yuechen Jiang, Yupeng Cao, Zhi Chen, Jordan W. Suchow, Zhenyu Cui, Rong Liu, Zhaozhuo Xu, Denghui Zhang, Koduvayur Subbalakshmi, Guojun Xiong, Yueru He, Jimin Huang, Dong Li, Qianqian Xie
**Date:** July 9, 2024 (presented December 2024)
**Venue:** **NeurIPS 2024** (main conference track), arXiv:2407.06567
**URL:** https://arxiv.org/abs/2407.06567

FinCon earned a main-track NeurIPS 2024 acceptance with a manager-analyst multi-agent hierarchy inspired by real investment firms. Seven distinct analyst agents (textual news, financial report, and numerical data processors) extract uni-modal investment insights, which a manager agent consolidates for final decisions. The key innovation is **Conceptual Verbal Reinforcement (CVRF)**, a dual-level risk-control mechanism that refines investment beliefs through self-critique of historical outcomes across episodes. This effectively allows the system to "learn from experience" **without gradient updates**—the agents build and update conceptual frameworks about what works in natural language. FinCon generalizes beyond single-stock trading to portfolio management. Using GPT-4-Turbo as backbone, experiments on U.S. equities show it outperforms both RL-based and other LLM-based agent systems in cumulative returns and Sharpe ratio. For intra-day system design, CVRF provides a blueprint for continuous improvement without model retraining—agents accumulate trading wisdom as text.

---

## 4. HedgeAgents' three conference types coordinate multi-asset hedging

**Title:** HedgeAgents: A Balanced-aware Multi-agent Financial Trading System
**Authors:** Xiangyu Li, Yawen Zeng, Xiaofen Xing, Jin Xu, Xiangmin Xu
**Date:** February 17, 2025 (accepted for oral presentation)
**Venue:** **The Web Conference 2025 (WWW '25)**, Companion Proceedings, arXiv:2502.13165
**URL:** https://arxiv.org/abs/2502.13165

HedgeAgents addresses a critical weakness the authors identified in existing LLM trading agents: **catastrophic losses during rapid market declines**, with typical systems losing 20%+ in volatile periods. The framework simulates a hedge fund with a central fund manager and three specialized hedging experts covering distinct asset classes (Bitcoin, Dow Jones, Forex). Each agent wields **23 tools and three types of memory** to execute eight distinct actions. Multi-agent coordination occurs through three conference types: **Budget Allocation Conferences** (resource distribution), **Experience Sharing Conferences** (cross-agent learning), and **Extreme Market Conferences** (emergency response during volatility spikes). Over a 3-year backtesting period, HedgeAgents achieved a **70% annualized return** and 400% total return with a Sharpe ratio of 2.41, outperforming all nine baselines. The conference-based coordination mechanism is the most novel architectural element—particularly the Extreme Market Conference, which triggers automatic defensive rebalancing, a pattern directly applicable to intra-day systems facing sudden volatility.

---

## 5. ElliottAgents brings wave pattern recognition into multi-agent territory

**Title:** ElliottAgents: Integrating Traditional Technical Analysis with AI — A Multi-Agent LLM-Based Approach to Stock Market Forecasting
**Authors:** Michał Wawer, Jarosław A. Chudziak
**Date:** June 20, 2025
**Venue:** arXiv preprint (cs.CE), arXiv:2506.16813; accepted at **ICAART 2025** (17th International Conference on Agents and Artificial Intelligence)
**URL:** https://arxiv.org/abs/2506.16813

ElliottAgents is the most explicit integration of classical technical analysis pattern recognition with multi-agent LLM architecture. The system uses specialized agents for data processing, **Elliott Wave pattern recognition** (identifying impulse 1-2-3-4-5 and corrective A-B-C sequences in price charts), and strategy formulation. The framework combines three AI techniques: LLMs for natural language understanding and decision-making, **Retrieval-Augmented Generation (RAG)** for accessing external knowledge bases of wave theory, and **Deep Reinforcement Learning (DRL)** for continuous learning. Built with LangGraph, agents collaborate dynamically using real-time market data via the yfinance API. Testing on historical data from major U.S. stocks demonstrated successful identification of impulse wave sequences and corrective patterns, generating actionable buy/sell signals across multiple timeframes. For an intra-day system, the RAG-augmented approach to pattern recognition—where agents can reference a knowledge base of pattern theory rather than relying solely on LLM pre-training—is particularly compelling.

---

## 6. Fine-grained task decomposition outperforms coarse agent roles

**Title:** Toward Expert Investment Teams: A Multi-Agent LLM System with Fine-Grained Trading Tasks
**Authors:** Kunihiro Miyazaki et al.
**Date:** February 2026
**Venue:** arXiv preprint, arXiv:2602.23330
**URL:** https://arxiv.org/abs/2602.23330

This paper addresses a largely overlooked design question: **how granular should agent task assignments be?** While most existing multi-agent trading systems assign coarse instructions (e.g., "analyze financial statements"), this work proposes explicit decomposition into fine-grained tasks mirroring real professional analyst workflows. The system uses a bottom-up manager-analyst framework with four specialist agents—Quantitative, Qualitative, News, and Technical—operating at Level 1 and feeding into higher-level synthesis. Evaluated on Japanese equity market data using GPT-4o with a leakage-controlled backtesting period (September 2023–November 2025), **fine-grained task decomposition significantly improved risk-adjusted returns over coarse-grained alternatives**. Comprehensive ablation studies systematically removing individual agents provide new insights into the relative contribution of each agent role. The core takeaway for system design: investing effort in granular task specification per agent matters more than simply adding more agents with vague mandates.

---

## 7. A taxonomy of 14 failure modes explains why multi-agent systems break

**Title:** Why Do Multi-Agent LLM Systems Fail?
**Authors:** Mert Cemri, Melissa Z. Pan, Shuyi Yang, Lakshya A. Agrawal, Bhavya Chopra, Rishabh Tiwari, Kurt Keutzer, Aditya Parameswaran, Dan Klein, Kannan Ramchandran, Matei Zaharia, Joseph E. Gonzalez, Ion Stoica (UC Berkeley, Stanford)
**Date:** March 17, 2025 (revised October 26, 2025)
**Venue:** **NeurIPS 2025 Datasets and Benchmarks Track (Spotlight)**, arXiv:2503.13657
**URL:** https://arxiv.org/abs/2503.13657

This paper is essential reading before building any multi-agent system. The authors introduce **MAST (Multi-Agent System Failure Taxonomy)**, the first systematic taxonomy of failure modes, built from **1,600+ annotated execution traces** across seven popular MAS frameworks. The taxonomy identifies 14 failure modes in three categories: **system design issues** (44.2% of failures, including agents disobeying role specifications, step repetition, and loss of conversation history), **inter-agent misalignment** (32.3%, including reasoning-action mismatch, information withholding between agents, ignored agent input, and task derailment), and **task verification failures** (23.5%, including premature termination and incorrect verification). Analysis reveals failure rates of **41% to 86.7%** across seven state-of-the-art open-source MAS frameworks. A critical finding: multi-agent performance gains on popular benchmarks are often minimal compared to single-agent frameworks and even simple best-of-N sampling. The **information withholding** failure mode (8.2% of failures)—where specialist agents fail to share critical analytical findings—is particularly dangerous for financial analysis systems where missing a single signal could be costly.

---

## 8. Reinforcement learning trains an orchestrator that discovers efficient agent workflows

**Title:** Multi-Agent Collaboration via Evolving Orchestration
**Authors:** Yufan Dang, Chen Qian, Xueheng Luo, Jingru Fan, Zihao Xie, Ruijie Shi, Weize Chen, Cheng Yang, Xiaoyin Che, Ye Tian, Xuantang Xiong, Lei Han, Zhiyuan Liu, Maosong Sun
**Date:** May 26, 2025
**Venue:** **NeurIPS 2025** (main conference), arXiv:2505.19591
**URL:** https://arxiv.org/abs/2505.19591

This paper proposes a "puppeteer-style" paradigm that directly addresses a core limitation of static multi-agent architectures. Rather than hand-designing fixed agent interaction sequences, a centralized **orchestrator is trained via reinforcement learning** to dynamically direct specialized agents in response to evolving task states. The orchestrator learns when to invoke which agent, how long to let agents iterate, and when to terminate—adapting its strategy based on task difficulty. Experiments show superior performance with **reduced computational costs** compared to static multi-agent designs. The most striking finding: the RL-trained orchestrator consistently discovers **compact, cyclic reasoning structures**, meaning it learns efficient patterns of agent interaction rather than brute-force sequential processing. For intra-day trading, where latency matters and market conditions shift rapidly, learned dynamic orchestration could adapt the analytical pipeline—invoking more agents during complex market conditions and fewer during clear trends—without manual reconfiguration.

---

## 9. Aegean brings distributed consensus theory to multi-agent LLM agreement

**Title:** Reaching Agreement Among Reasoning LLM Agents
**Authors:** Chaoyi Ruan, Yiliang Wang, Ziji Shi, Jialin Li (National University of Singapore)
**Date:** December 23, 2025
**Venue:** arXiv preprint, arXiv:2512.20184
**URL:** https://arxiv.org/abs/2512.20184

Aegean formalizes the multi-agent agreement problem by drawing on **classical distributed consensus theory** (Paxos, Raft) and applying it to stochastic LLM reasoning agents. The authors argue that existing multi-agent orchestration relies on ad-hoc heuristics—fixed loop limits, barrier synchronization—that waste compute, incur straggler latency, and risk finalizing unstable agreements. They formalize the **Multi-Agent Refinement Problem** with three correctness guarantees: Refinement Termination (a correct agent eventually outputs a solution), Refinement Validity (output quality matches or exceeds independent majority), and **Refinement Monotonicity** (quality never degrades across rounds). The Aegean protocol uses leader-based coordination with quorum detection: a leader agent collects solutions from a quorum, broadcasts reference sets for refinement, and finalizes only when agreement is **stable for 2+ consecutive rounds**. Evaluation on mathematical reasoning benchmarks shows **1.2–20× latency reduction** versus baselines while maintaining answer quality. For a trading system where multiple analyst agents must converge on a market view, Aegean provides formal guarantees that the system won't finalize a premature or unstable consensus—a critical property when real capital is at stake.

---

## 10. Debate adds less value than voting in multi-agent systems—with caveats

**Title:** Debate or Vote: Which Yields Better Decisions in Multi-Agent Large Language Models?
**Authors:** Hyeong Kyu Choi, Xiaojin Zhu, Sharon Li (University of Wisconsin-Madison)
**Date:** August 24, 2025 (revised October 23, 2025)
**Venue:** arXiv preprint, arXiv:2508.17536
**URL:** https://arxiv.org/abs/2508.17536

This paper delivers a crucial theoretical and empirical caution for anyone designing multi-agent debate mechanisms (like TradingAgents' bull-bear debate). The authors disentangle Multi-Agent Debate (MAD) into two components—**majority voting and inter-agent debate**—and assess their respective contributions across seven NLP benchmarks. The striking finding: **majority voting alone accounts for most of the performance gains** typically attributed to debate. A theoretical framework models debate as a stochastic process, proving it induces a **martingale over agents' belief trajectories**, meaning debate alone does not improve expected correctness in theory. However, the paper identifies an important exception: **targeted interventions biasing belief updates toward correction** can meaningfully enhance debate. For financial analysis, this suggests that naive "bull vs. bear" debate may be less valuable than independent parallel analysis followed by weighted aggregation. But tasks requiring genuine synthesis of heterogeneous information—combining technical, fundamental, and sentiment signals into a trading decision—may benefit from structured debate more than tasks with a single correct answer. The design implication is clear: build consensus mechanisms that add information, not just argumentation.

---

## Architectural patterns emerging across the literature

Several design principles recur consistently across these papers and carry direct implications for building an intra-day multi-agent system.

**Specialization wins, but granularity matters.** Every paper that tested specialized agents against generalist alternatives found specialization superior. Miyazaki et al. (Paper 6) go further, showing that **fine-grained task specifications within each specialist role** produce significantly better risk-adjusted returns than coarse-grained mandates. The practical implication: rather than an agent prompted to "do technical analysis," create agents for specific sub-tasks—RSI divergence detection, support/resistance identification, volume profile analysis—each with precise instructions and domain-specific tools.

**Coordination architecture makes or breaks the system.** The failure taxonomy from MAST (Paper 7) reveals that **76.5% of multi-agent failures** stem from either system design issues or inter-agent misalignment, not from individual agent incompetence. HedgeAgents' three conference types (Paper 4) and Aegean's formal consensus protocol (Paper 9) offer complementary solutions: structured coordination events for routine synthesis, and formal agreement guarantees for high-stakes decisions. The evolving orchestration approach (Paper 8) suggests that the optimal coordination pattern may itself be learned rather than hand-designed.

**Memory and learning without retraining.** FinCon's Conceptual Verbal Reinforcement (Paper 3) and HedgeAgents' Experience Sharing Conferences (Paper 4) both demonstrate that multi-agent systems can accumulate and apply trading experience through natural language—updating beliefs about what works without gradient updates. For an intra-day system, this enables continuous adaptation to changing market regimes through conceptual updates rather than model retraining.

**Price action as structured reasoning.** QuantAgent (Paper 1) and ElliottAgents (Paper 5) make the strongest case that technical analysis—pattern recognition, indicator interpretation, and price structure analysis—maps naturally onto LLM reasoning capabilities. The key insight is that price action analysis involves **structured, rule-based interpretation** of visual and numerical patterns, which LLMs can handle through tool-augmented reasoning chains when given appropriate decomposition and domain-specific tools.