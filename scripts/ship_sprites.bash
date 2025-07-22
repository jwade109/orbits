for i in 1.0 0.95 0.90 0.85 0.80 0.75 0.70 0.65 0.60 0.55 0.50 0.45 0.40 0.35 0.30 0.25 0.20 0.15 0.10 0.05
do
    echo $i;
    ../target/release/ship2png -p ../assets/parts/ -s ../assets/vehicles/mule.vehicle -o ship-$i.png -g 10 -x $i
done
