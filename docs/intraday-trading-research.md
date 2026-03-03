# Essential papers for ML-driven equity trading research

**This curated bibliography compiles 32 of the most important academic papers across four pillars of quantitative intraday equity trading: ML-based strategies, candlestick pattern analysis, multi-timeframe modeling, and market microstructure.** The collection spans foundational theoretical works from the 1980s through cutting-edge deep learning research published in 2025, with strong emphasis on reproducible methods, clear backtests, and papers that have shaped how practitioners actually build trading systems. Papers are organized by topic, with key metadata and brief assessments of each work's contribution.

---

## Intraday ML trading strategies that define the field

These papers establish the methodological backbone for short-horizon equity prediction using machine learning, reinforcement learning, and systematic feature engineering.

**1. Empirical Asset Pricing via Machine Learning**
Shihao Gu, Bryan Kelly, Dacheng Xiu (2020). *The Review of Financial Studies*, 33(5), 2223–2273. The definitive comparative benchmark for ML methods in equity return prediction. Evaluates penalized linear models, random forests, gradient boosting, and neural networks across **~94 stock-level characteristics**, demonstrating that nonlinear models roughly double Sharpe ratios versus linear benchmarks. While focused on monthly horizons, this paper established the methodological standard — train/validate/test splits, hyperparameter protocols, out-of-sample evaluation — that all subsequent intraday ML research builds upon. DOI: [10.1093/rfs/hhaa009](https://doi.org/10.1093/rfs/hhaa009)

**2. Advances in Financial Machine Learning**
Marcos López de Prado (2018). *Wiley*. The practitioner-academic reference that introduced the **triple-barrier labeling method**, **meta-labeling**, information-driven bars (volume, tick, dollar bars), fractional differentiation for stationarity, and purged k-fold cross-validation. These techniques are now standard in professional quant pipelines for intraday strategy development. Open-source implementations available via the `mlfinlab` library (Hudson & Thames). URL: [wiley.com](https://www.wiley.com/en-us/Advances+in+Financial+Machine+Learning-p-9781119482086)

**3. Intraday Market Predictability: A Machine Learning Approach**
Dillon Huddleston, Fred Liu, Lars Stentoft (2023). *Journal of Financial Econometrics*, 21(2), 485–527. Arguably the largest study of five-minute equity market return predictability using ML. Ensemble models achieve **Sharpe ratios of ~0.98 after transaction costs** using Lasso, Elastic Net, Random Forest, Gradient Boosting, and Neural Networks trained on lagged cross-sectional constituent returns. Finds predictability is driven by slow-moving capital and is more pronounced in less liquid periods. DOI: [10.1093/jjfinec/nbab007](https://doi.org/10.1093/jjfinec/nbab007)

**4. Intraday Stock Predictability Everywhere**
Fred Liu, Lars Stentoft (2023). *SSRN Working Paper*. Extends the authors' prior work to ~900 million observations across individual stock returns at various intraday horizons. Nonlinear models economically dominate linear models, with ML-based intraday long-short portfolios attaining **Sharpe ratios of ~4 after transaction costs**. Predictability is short-lived, highest mid-day, and more pronounced for less liquid firms. URL: [papers.ssrn.com/sol3/papers.cfm?abstract_id=4496917](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=4496917)

**5. DeepScalper: A Risk-Aware Reinforcement Learning Framework to Capture Fleeting Intraday Trading Opportunities**
Shuo Sun, Wanqi Xue, Rundong Wang, Xu He, Junlei Zhu, Jian Li, Bo An (2022). *CIKM '22*. A deep RL framework for minute-level intraday trading featuring a dueling Q-network with action branching, hindsight bonus rewards, encoder-decoder architecture incorporating macro and LOB data, and a risk-aware volatility prediction auxiliary task. Significantly outperforms baselines across multiple asset classes. DOI: [10.1145/3511808.3557283](https://doi.org/10.1145/3511808.3557283)

**6. Deep Reinforcement Learning for Trading**
Zihao Zhang, Stefan Zohren, Stephen Roberts (2020). *The Journal of Financial Data Science*, 2(2), 25–40. From the Oxford-Man Institute: applies DQN, DDQN, and actor-critic algorithms with volatility-scaled reward functions to 50 liquid futures contracts (including equity indices). A methodologically clean reference for applying RL to short-horizon trading with proper transaction cost modeling. URL: [arxiv.org/abs/1911.10107](https://arxiv.org/abs/1911.10107)

**7. Intra-day Equity Price Prediction using Deep Learning as a Measure of Market Efficiency**
David Byrd, Tucker Hybinette Balch (2019). *arXiv:1908.08168* (Georgia Tech / J.P. Morgan AI Research). Demonstrates that intraday stock prices were predictable using ML until approximately **2009**, after which profitability vanished — aligning with the rise of HFT. Provides a rigorous 15-year backtest (2003–2017) and proposes using ML profitability as an objective measure of relative market efficiency. URL: [arxiv.org/abs/1908.08168](https://arxiv.org/abs/1908.08168)

**8. Deep Reinforcement Learning with Positional Context for Intraday Trading**
Sven Goluža, Toni Kovačević, Teo Bauman et al. (2024). *Evolving Systems*, 15, 1865–1880 (Springer). Enriches the RL state space with positional context features (current position, elapsed time, P&L state) alongside standard price indicators. Evaluated over nearly a decade across multiple asset classes with realistic transaction costs, benchmarked against DeepScalper. DOI: [10.1007/s12530-024-09593-6](https://doi.org/10.1007/s12530-024-09593-6)

---

## Candlestick patterns under the microscope of ML and statistics

This section spans rigorous statistical debunking studies, ML-enhanced pattern recognition, and deep learning approaches that treat candlestick charts as images. The literature is notably split between papers that confirm and deny pattern efficacy.

**9. Candlestick Technical Trading Strategies: Can They Create Value for Investors?**
Ben R. Marshall, Martin R. Young, Lawrence C. Rose (2006). *Journal of Banking & Finance*, 30(8), 2303–2323. The seminal debunking study. Tests **28 candlestick patterns** on DJIA stocks (1992–2002) using an innovative bootstrap methodology generating random OHLC series. Finds candlestick strategies have no statistically significant value, robust across 9 assumption scenarios. The most cited empirical challenge to candlestick pattern efficacy and the methodological benchmark for all subsequent studies. DOI: [10.1016/j.jbankfin.2005.08.001](https://doi.org/10.1016/j.jbankfin.2005.08.001)

**10. The Predictive Power of Japanese Candlestick Charting in Chinese Stock Market**
Shi Chen, Si Bao, Yu Zhou (2016). *Physica A*, 457, 148–165. Rigorous examination of four pairs of bullish/bearish patterns on Chinese stocks (2007–2015) using the Step-SPA test to correct for data-snooping bias. Nuanced results: **5 of 8 patterns show significance**, but power decays with horizon and is stronger for medium-cap stocks. Provides formal quantitative definitions of candlestick patterns. DOI: [10.1016/j.physa.2016.03.081](https://doi.org/10.1016/j.physa.2016.03.081)

**11. Profitability of Candlestick Charting Patterns in the Stock Exchange of Thailand**
Piyapas Tharavanij, Vasan Siraprapasiri, Kittichai Rajchamaha (2017). *SAGE Open*, 7(4). Comprehensive evaluation using SET50 Thai stocks with multiple holding periods, two exit strategies, and technical filtering. Uses skewness-adjusted t-tests and binomial tests. Finds most reversal patterns do not generate statistically significant returns; technical filters generally do not improve results. DOI: [10.1177/2158244017736799](https://doi.org/10.1177/2158244017736799)

**12. Trading via Image Classification**
Naftali Cohen, Tucker Balch, Manuela Veloso (2019/2020). *ICAIF 2020* / arXiv:1907.10046 (J.P. Morgan AI Research). Transforms financial time-series classification into image classification. Creates candlestick chart images from S&P 500 data and trains CNNs to identify trade signals (Bollinger, MACD, RSI). Achieves **~95% accuracy** for Bollinger/RSI signals from 30×30 pixel images, proving chart-pattern signals are machine-recoverable vision features. DOI: [10.1145/3383455.3422544](https://doi.org/10.1145/3383455.3422544)

**13. Encoding Candlesticks as Images for Pattern Classification Using Convolutional Neural Networks**
Jun-Hao Chen, Yun-Cheng Tsai (2020). *Financial Innovation*, 6, Article 26 (Springer). The foundational **Gramian Angular Field (GAF)** paper for finance. Encodes OHLC time-series data as 2D images, then trains a CNN (LeNet-based) to classify 8 candlestick patterns, outperforming LSTM baselines. Established GAF encoding as a standard technique for converting financial time series to image representations. DOI: [10.1186/s40854-020-00187-0](https://doi.org/10.1186/s40854-020-00187-0)

**14. Improving Stock Trading Decisions Based on Pattern Recognition Using Machine Learning Technology**
Yaohu Lin, Shancun Liu, Haijun Yang, Harris Wu, Bingbing Jiang (2021). *PLoS ONE*, 16(8), e0255558. The PRML framework applies four ML methods to all possible 1-to-3-day candlestick pattern combinations on the full Chinese stock market (2000–2020). Two-day patterns after ML filtering achieve **36.73% average annual return with Sharpe ratio of 0.81**, making this the most comprehensive ML-based candlestick pattern study. DOI: [10.1371/journal.pone.0255558](https://doi.org/10.1371/journal.pone.0255558)

**15. Deep Reinforcement Learning Stock Market Trading, Utilizing a CNN with Candlestick Images**
Andrew Brim, Nicholas S. Flann (2022). *PLoS ONE*, 17(2), e0263181. Combines CNN with a DDQN reinforcement learning agent using candlestick images as sole input. Tested on 30 largest S&P 500 stocks during the COVID-19 crash period. Uses feature map visualizations showing the CNN shifts attention to the most recent candles during volatile periods, providing rare interpretability for CNN-based candlestick trading. DOI: [10.1371/journal.pone.0263181](https://doi.org/10.1371/journal.pone.0263181)

**16. Dynamic Deep Convolutional Candlestick Learner**
Jun-Hao Chen, Yun-Cheng Tsai (2022). *arXiv:2201.08669*. Extends the GAF-CNN approach to object detection using a modified **YOLOv1 architecture** for simultaneous candlestick pattern classification and localization within chart images. A step toward real-time automated pattern detection systems. URL: [arxiv.org/abs/2201.08669](https://arxiv.org/abs/2201.08669)

---

## Multi-timeframe models that fuse temporal resolutions

A rapidly growing subfield demonstrating that explicitly combining signals from multiple temporal scales — via wavelets, attention mechanisms, or hierarchical architectures — consistently outperforms single-scale approaches.

**17. An Introduction to Wavelets and Other Filtering Methods in Finance and Economics**
Ramazan Gençay, Faruk Selçuk, Brandon J. Whitcher (2001). *Academic Press*. The foundational reference for wavelet analysis in finance. Presents multi-resolution analysis (MRA) for decomposing price series into different temporal scales, revealing time-varying volatility clusters, structural breaks, and scale-dependent correlations — the theoretical underpinning for all subsequent multi-scale financial ML. ISBN: 978-0122796708

**18. Stock Price Prediction via Discovering Multi-Frequency Trading Patterns**
Liheng Zhang, Charu Aggarwal, Guo-Jun Qi (2017). *KDD 2017*. Introduces the **State Frequency Memory (SFM)** recurrent network, decomposing LSTM hidden states into multiple frequency components via Discrete Fourier Transform. High-frequency components capture short-term volatility while low-frequency components model trends. A landmark demonstration that explicit frequency-band separation yields superior prediction over standard LSTM. DOI: [10.1145/3097983.3098117](https://doi.org/10.1145/3097983.3098117)

**19. Hierarchical Multi-Scale Gaussian Transformer for Stock Movement Prediction**
Qianggang Ding, Sifan Wu, Hao Sun, Jiadong Guo, Jian Guo (2020). *IJCAI 2020*. A Transformer with multi-scale Gaussian prior biases on attention matrices, enabling the model to attend to different temporal horizons (5, 10, 20, 40-day windows). Orthogonal Regularization prevents redundant attention heads, and a Trading Gap Splitter separates intra-day from intra-week features. Achieves state-of-the-art stock movement prediction. URL: [ijcai.org/proceedings/2020/0640.pdf](https://www.ijcai.org/proceedings/2020/0640.pdf)

**20. Multi-Scale Two-Way Deep Neural Network for Stock Trend Prediction (MTDNN)**
Guang Liu, Yuzhao Mao, Qi Sun et al. (2020). *IJCAI 2020*. A dual-pathway architecture: one pathway uses **Discrete Wavelet Transform (DWT)** to decompose prices into multi-resolution components ensembled via XGBoost; the other uses temporal downsampling at multiple scales through a Recurrent CNN. Strong evidence that multi-scale information significantly improves prediction. Code available on GitHub. URL: [ijcai.org/proceedings/2020/0628.pdf](https://www.ijcai.org/proceedings/2020/0628.pdf)

**21. Stock Trend Prediction with Multi-Granularity Data: A Contrastive Learning Approach with Adaptive Fusion (CMLF)**
Min Hou, Chang Xu, Yang Liu et al. (2021). *CIKM 2021*. Directly tackles fusing **minute-level and daily-frequency** stock data using contrastive learning objectives — Cross-Granularity (local) and Cross-Temporal (global) — to bridge fine-grained and coarse-grained representations. An adaptive gate mechanism handles dynamic multi-granularity fusion. Evaluated on CSI300, CSI800, and NASDAQ100, showing significant gains from multi-granularity data. DOI: [10.1145/3459637.3482483](https://doi.org/10.1145/3459637.3482483)

**22. Hierarchical Adaptive Temporal-Relational Modeling for Stock Trend Prediction (HATR)**
Heyuan Wang, Shun Li, Tengjiao Wang, Jiayi Zheng (2021). *IJCAI 2021*. Stacked dilated causal convolutions (dilation rates 1-2-4) progressively capture multi-scale temporal patterns. Dual attention — point-wise temporal and scale-wise — jointly focuses on salient time points and the most informative scales. The learned multi-scale representations feed into a relational module for stock interdependency modeling. URL: [ijcai.org/proceedings/2021/0508.pdf](https://www.ijcai.org/proceedings/2021/0508.pdf)

**23. Multi-Granularity Spatio-Temporal Correlation Networks for Stock Trend Prediction**
Jiahao Chen, Qiao Qin, Yunfeng Zhang et al. (2024). *IEEE Access*. Constructs multiple temporal granularities with dedicated GRU + Graph Attention layers at each level, plus a cross-granularity residual learning mechanism. Tested on CSI 300 and NASDAQ 100 (2010–2023), demonstrating **~80% excess return** over the CSI 300 index. DOI: [10.1109/ACCESS.2024.3393774](https://doi.org/10.1109/ACCESS.2024.3393774)

**24. Multi-Scale Temporal Neural Network for Stock Trend Prediction Enhanced by Temporal Hyperedge Learnings (MSTNN)**
Litsong Sun et al. (2025). *IJCAI 2025*. The most recent top-venue paper in this area. Uses a 3D Multi-Channel CNN with filters of varying dimensions to identify periodic patterns across weekly, monthly, and annual scales, paired with a Temporal Hypergraph Attention Network for group-wise stock dynamics. Ablating the multi-scale CNN component drops F1 by **27–34%**, confirming the critical importance of multi-scale temporal modeling. Code available on GitHub. URL: [ijcai.org/proceedings/2025/0364.pdf](https://www.ijcai.org/proceedings/2025/0364.pdf)

---

## Microstructure and order flow as prediction engines

From the foundational theories of information-based trading to modern deep learning on limit order books, these papers provide the theoretical and empirical framework for extracting predictive signals from market microstructure data.

**25. Continuous Auctions and Insider Trading**
Albert S. Kyle (1985). *Econometrica*, 53(6), 1315–1335. The foundational microstructure model. Derives **Kyle's lambda** — a linear price impact proportional to order flow — from a strategic informed trader framework. Establishes the theoretical backbone for all subsequent work on information-based trading and price impact. DOI: [10.2307/1913210](https://doi.org/10.2307/1913210)

**26. Bid, Ask and Transaction Prices in a Specialist Market with Heterogeneously Informed Traders**
Lawrence R. Glosten, Paul R. Milgrom (1985). *Journal of Financial Economics*, 14(1), 71–100. The bid-ask spread arises endogenously from **adverse selection**: each trade causes Bayesian updating about asset value. Paired with Kyle (1985), forms the dual pillar of information-based microstructure theory. DOI: [10.1016/0304-405X(85)90044-3](https://doi.org/10.1016/0304-405X(85)90044-3)

**27. Optimal Execution of Portfolio Transactions**
Robert Almgren, Neil Chriss (2001). *Journal of Risk*, 3(2), 5–40. The canonical optimal execution model separating **permanent and temporary market impact** and deriving the efficient frontier of liquidation strategies trading off expected cost against execution risk. Foundational for algorithmic execution, VWAP/TWAP benchmarking, and intraday volume profile modeling. URL: [semanticscholar.org](https://www.semanticscholar.org/paper/Optimal-execution-of-portfolio-trans-actions-Almgren-Chriss/4ea1885d7f00dc2ba59be2d6cc62923de23599ce)

**28. Flow Toxicity and Liquidity in a High-Frequency World (VPIN)**
David Easley, Marcos López de Prado, Maureen O'Hara (2012). *Review of Financial Studies*, 25(5), 1457–1493. Introduces **VPIN** (Volume-Synchronized Probability of Informed Trading), a real-time order flow toxicity measure using volume-time sampling and bulk volume classification. VPIN famously spiked before the May 2010 Flash Crash. Critical for understanding and predicting liquidity risk in real time. DOI: [10.1093/rfs/hhs053](https://doi.org/10.1093/rfs/hhs053)

**29. The Price Impact of Order Book Events**
Rama Cont, Arseniy Kukanov, Sasha Stoikov (2014). *Journal of Financial Econometrics*, 12(1), 47–88. The landmark empirical study of **order flow imbalance (OFI)** as a predictor. Using NYSE data for 50 U.S. stocks, shows price changes are primarily driven by OFI with a linear impact relationship inversely proportional to market depth. This paper operationalized OFI as a key microstructure feature used in virtually all subsequent ML-based LOB prediction work. DOI: [10.1093/jjfinec/nbt003](https://doi.org/10.1093/jjfinec/nbt003)

**30. Hawkes Processes in Finance**
Emmanuel Bacry, Iacopo Mastromatteo, Jean-François Muzy (2015). *Market Microstructure and Liquidity*, 1(1), 1550005. Comprehensive review of self-exciting point processes for modeling order arrival clustering and mutual excitation in limit order books. Covers volatility estimation, market stability, optimal execution, and full LOB dynamics. Essential background for point-process models of order flow. DOI: [10.1142/S2382626615500057](https://doi.org/10.1142/S2382626615500057)

**31. DeepLOB: Deep Convolutional Neural Networks for Limit Order Books**
Zihao Zhang, Stefan Zohren, Stephen Roberts (2019). *IEEE Transactions on Signal Processing*, 67(11), 3001–3012. The benchmark deep learning architecture for LOB prediction, combining CNNs (spatial LOB structure) with LSTMs (temporal dependencies) for mid-price movement prediction. Achieves state-of-the-art on FI-2010 and generalizes to unseen instruments on LSE data. **Open-source PyTorch code available.** DOI: [10.1109/TSP.2019.2907260](https://doi.org/10.1109/TSP.2019.2907260)

**32. Universal Features of Price Formation in Financial Markets: Perspectives from Deep Learning**
Justin Sirignano, Rama Cont (2019). *Quantitative Finance*, 19(9), 1449–1459. Applies large-scale deep learning to **billions of quotes and transactions** for U.S. equities, uncovering a universal, stationary relationship between order flow history and price movement. The universal cross-stock model outperforms asset-specific models, arguing strongly for cross-sectional pooling. Provides evidence of path-dependence in order flow. DOI: [10.1080/14697688.2019.1622295](https://doi.org/10.1080/14697688.2019.1622295)

**33. Deep Order Flow Imbalance: Extracting Alpha at Multiple Horizons from the Limit Order Book**
Petter N. Kolm, Jeremy Turiel, Nicholas Westray (2023). *Mathematical Finance*, 33(4), 1044–1081. Deep learning (LSTM, CNN, MLP) forecasts returns at multiple horizons for 115 NASDAQ stocks. Models trained on **stationary OFI features significantly outperform** models on raw LOB states. Introduces the "alpha term structure" concept — a vector of return forecasts at multiple horizons. Related work won Risk.net's Buy-Side Quant of the Year 2025. DOI: [10.1111/mafi.12413](https://doi.org/10.1111/mafi.12413)

**34. Cross-Impact of Order Flow Imbalance in Equity Markets**
Rama Cont, Mihai Cucuringu, Chao Zhang (2023). *Quantitative Finance*, 24(1), 1–21. Extends OFI to a multi-asset, multi-level setting. Shows that **lagged cross-asset OFIs significantly improve return forecasting** at short horizons, and that multi-level OFI (integrating across LOB depth) better explains price impact than best-level OFI alone. Important for portfolio-level microstructure prediction. DOI: [10.1080/14697688.2023.2236159](https://doi.org/10.1080/14697688.2023.2236159)

---

## Connecting the threads across all four domains

Several themes emerge from this literature that cut across all four categories. First, **nonlinear models consistently dominate linear ones** for short-horizon equity prediction — this finding appears in Gu, Kelly & Xiu (2020), Liu & Stentoft (2023), and Sirignano & Cont (2019) across very different data and methodologies. Second, **feature engineering matters more than model architecture**: Kolm et al. (2023) show that stationary OFI features dramatically outperform raw LOB states regardless of model choice, and López de Prado's (2018) information-driven bars have become standard because they improve signal quality before any ML is applied.

Third, the multi-timeframe literature provides strong, consistent evidence that **explicitly modeling multiple temporal resolutions improves prediction** — ablation studies in MSTNN (2025) show 27–34% F1 drops from removing multi-scale components. This suggests that combining microstructure signals (seconds-to-minutes) with pattern-based signals (minutes-to-hours) and trend signals (hours-to-days) through architectures like CMLF or HATR is a promising direction.

Finally, the candlestick literature reveals an important nuance: while traditional patterns show **limited standalone predictive power** in efficient markets (Marshall et al. 2006), ML-filtered pattern combinations can extract significant alpha (Lin et al. 2021), and CNN-based image approaches recover signals that rule-based methods miss (Cohen et al. 2019). The gap between these findings suggests that candlestick patterns encode real but noisy information that requires ML to extract reliably — a conclusion consistent with the broader theme that nonlinear methods unlock value invisible to traditional approaches.

For practitioners building intraday equity trading systems, the most actionable starting points are López de Prado (2018) for pipeline design, Cont et al. (2014) for feature engineering from order flow, DeepLOB (2019) for LOB modeling, and the CMLF framework (2021) for multi-granularity signal fusion. Code availability for DeepLOB, MTDNN, and MSTNN makes these particularly suitable for reproducible implementation.